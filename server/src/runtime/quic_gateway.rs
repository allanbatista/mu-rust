use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context};
use protocol::{preferred_channel, TransportKind, WireCodec, WirePacket};
use quinn::{Connection, Endpoint, RecvStream, SendStream};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use super::config::GatewayConfig;
use super::MuCoreRuntime;

#[derive(Debug, Clone)]
pub struct QuicTlsPaths {
    pub cert: PathBuf,
    pub key: PathBuf,
}

#[derive(Clone)]
pub struct QuicGatewayHandle {
    endpoint: Endpoint,
    local_addr: SocketAddr,
}

impl QuicGatewayHandle {
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"server shutdown");
    }
}

pub async fn start_quic_gateway(
    runtime: Arc<MuCoreRuntime>,
    gateway: &GatewayConfig,
    tls_paths: Option<QuicTlsPaths>,
) -> anyhow::Result<QuicGatewayHandle> {
    let bind_addr = format!("{}:{}", gateway.host, gateway.port)
        .parse::<SocketAddr>()
        .with_context(|| {
            format!(
                "invalid QUIC gateway bind address '{}:{}'",
                gateway.host, gateway.port
            )
        })?;

    let (cert_chain, private_key) = load_tls_material(tls_paths.as_ref())?;

    let mut server_config = quinn::ServerConfig::with_single_cert(cert_chain, private_key)
        .context("invalid TLS material for QUIC")?;

    let transport = Arc::get_mut(&mut server_config.transport)
        .ok_or_else(|| anyhow!("unable to mutate QUIC transport config"))?;

    transport.max_concurrent_bidi_streams(quinn::VarInt::from_u32(2_048));
    transport.max_concurrent_uni_streams(quinn::VarInt::from_u32(2_048));
    transport.keep_alive_interval(Some(Duration::from_secs(5)));
    transport.max_idle_timeout(Some(quinn::IdleTimeout::try_from(Duration::from_secs(30))?));
    transport.datagram_receive_buffer_size(Some(4 * 1024 * 1024));
    transport.datagram_send_buffer_size(4 * 1024 * 1024);

    let endpoint =
        Endpoint::server(server_config, bind_addr).context("failed to create QUIC endpoint")?;
    let local_addr = endpoint
        .local_addr()
        .context("failed to resolve QUIC local address")?;

    let accept_endpoint = endpoint.clone();
    tokio::spawn(async move {
        accept_loop(accept_endpoint, runtime).await;
    });

    Ok(QuicGatewayHandle {
        endpoint,
        local_addr,
    })
}

async fn accept_loop(endpoint: Endpoint, runtime: Arc<MuCoreRuntime>) {
    loop {
        let Some(incoming) = endpoint.accept().await else {
            break;
        };

        let runtime_clone = runtime.clone();
        tokio::spawn(async move {
            match incoming.await {
                Ok(connection) => {
                    log::info!("QUIC client connected from {}", connection.remote_address());
                    handle_connection(connection, runtime_clone).await;
                }
                Err(err) => {
                    log::warn!("QUIC handshake failed: {}", err);
                }
            }
        });
    }
}

async fn handle_connection(connection: Connection, runtime: Arc<MuCoreRuntime>) {
    let stream_task = tokio::spawn(handle_bidi_streams(connection.clone(), runtime.clone()));
    let datagram_task = tokio::spawn(handle_datagrams(connection.clone(), runtime));

    let _ = tokio::join!(stream_task, datagram_task);

    log::info!(
        "QUIC client disconnected from {}",
        connection.remote_address()
    );
}

async fn handle_bidi_streams(connection: Connection, runtime: Arc<MuCoreRuntime>) {
    let codec = WireCodec::default();

    loop {
        let (mut send, mut recv) = match connection.accept_bi().await {
            Ok(streams) => streams,
            Err(err) => {
                log::debug!(
                    "QUIC stream accept ended for {}: {}",
                    connection.remote_address(),
                    err
                );
                break;
            }
        };

        let runtime_clone = runtime.clone();
        let codec_clone = codec.clone();

        tokio::spawn(async move {
            if let Err(err) =
                handle_single_bidi_stream(&runtime_clone, &codec_clone, &mut recv, &mut send).await
            {
                log::debug!("QUIC stream handling error: {}", err);
            }
        });
    }
}

async fn handle_single_bidi_stream(
    runtime: &Arc<MuCoreRuntime>,
    codec: &WireCodec,
    recv: &mut RecvStream,
    send: &mut SendStream,
) -> anyhow::Result<()> {
    let max_read_size = codec
        .limits()
        .max_stream_payload_size
        .saturating_mul(8)
        .max(1024);

    let bytes = recv
        .read_to_end(max_read_size)
        .await
        .context("failed to read QUIC stream payload")?;

    if bytes.is_empty() {
        send.finish()
            .map_err(|e| anyhow!("failed to finish empty stream: {}", e))?;
        return Ok(());
    }

    let server_time_ms = now_ms();
    let responses = runtime
        .handle_stream_bytes(&bytes, server_time_ms)
        .await
        .context("failed to process stream bytes")?;

    for packet in responses {
        write_packet_to_stream(codec, send, &packet).await?;
    }

    send.finish()
        .map_err(|e| anyhow!("failed to finish response stream: {}", e))?;

    Ok(())
}

async fn write_packet_to_stream(
    codec: &WireCodec,
    send: &mut SendStream,
    packet: &WirePacket,
) -> anyhow::Result<()> {
    let channel = preferred_channel(&packet.payload);

    if channel.transport() == TransportKind::Datagram {
        // Datagram payloads are not emitted in stream-bound responses.
        return Ok(());
    }

    let frame = codec
        .encode_stream_frame(channel, packet)
        .context("failed to encode stream frame")?;

    send.write_all(&frame)
        .await
        .context("failed to write stream frame")?;

    Ok(())
}

async fn handle_datagrams(connection: Connection, runtime: Arc<MuCoreRuntime>) {
    let codec = WireCodec::default();

    loop {
        let datagram = match connection.read_datagram().await {
            Ok(bytes) => bytes,
            Err(err) => {
                log::debug!(
                    "QUIC datagram loop ended for {}: {}",
                    connection.remote_address(),
                    err
                );
                break;
            }
        };

        let response = match runtime
            .handle_datagram_frame(datagram.as_ref(), now_ms())
            .await
        {
            Ok(response) => response,
            Err(err) => {
                log::debug!("QUIC datagram decode/dispatch error: {}", err);
                continue;
            }
        };

        if let Some(packet) = response {
            if let Err(err) = send_packet_over_connection(&connection, &codec, &packet).await {
                log::debug!("QUIC datagram response send failed: {}", err);
            }
        }
    }
}

async fn send_packet_over_connection(
    connection: &Connection,
    codec: &WireCodec,
    packet: &WirePacket,
) -> anyhow::Result<()> {
    let channel = preferred_channel(&packet.payload);

    match channel.transport() {
        TransportKind::Datagram => {
            let frame = codec
                .encode_datagram_frame(channel, packet)
                .context("failed to encode datagram frame")?;

            connection
                .send_datagram(frame.into())
                .map_err(|e| anyhow!("failed to send datagram response: {}", e))?;
        }
        _ => {
            let frame = codec
                .encode_stream_frame(channel, packet)
                .context("failed to encode stream frame")?;

            let mut send = connection
                .open_uni()
                .await
                .map_err(|e| anyhow!("failed to open uni stream: {}", e))?;

            send.write_all(&frame)
                .await
                .context("failed to write uni stream frame")?;

            send.finish()
                .map_err(|e| anyhow!("failed to finish uni stream: {}", e))?;
        }
    }

    Ok(())
}

fn load_tls_material(
    tls_paths: Option<&QuicTlsPaths>,
) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    match tls_paths {
        Some(paths) => load_tls_from_files(paths),
        None => generate_self_signed_tls(),
    }
}

fn load_tls_from_files(
    tls_paths: &QuicTlsPaths,
) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let cert_bytes = std::fs::read(&tls_paths.cert).with_context(|| {
        format!(
            "failed to read QUIC certificate file '{}'",
            tls_paths.cert.display()
        )
    })?;

    let key_bytes = std::fs::read(&tls_paths.key)
        .with_context(|| format!("failed to read QUIC key file '{}'", tls_paths.key.display()))?;

    let mut cert_reader = Cursor::new(cert_bytes);
    let cert_chain = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .context("failed to parse QUIC certificate chain")?;

    if cert_chain.is_empty() {
        bail!("QUIC certificate chain is empty");
    }

    let mut key_reader = Cursor::new(key_bytes);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .context("failed to parse QUIC private key")?
        .ok_or_else(|| anyhow!("no private key found in QUIC key file"))?;

    Ok((cert_chain, key))
}

fn generate_self_signed_tls(
) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let certified = rcgen::generate_simple_self_signed(subject_alt_names)
        .context("failed to generate self-signed certificate")?;

    let cert_der = certified.cert.der().clone();
    let key_der = PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der());

    Ok((vec![cert_der], PrivateKeyDer::Pkcs8(key_der)))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_self_signed_tls_material() {
        let (certs, key) = generate_self_signed_tls().expect("must generate cert and key");
        assert!(!certs.is_empty());

        match key {
            PrivateKeyDer::Pkcs8(_) => {}
            _ => panic!("expected pkcs8 key"),
        }
    }
}
