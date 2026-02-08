use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context};
use protocol::{
    ClientHello, ClientMessage, PacketPayload, QuicChannel, RouteKey, ServerMessage, WireCodec,
    WirePacket,
};
use quinn::Endpoint;
use reqwest::StatusCode;
use rustls::pki_types::CertificateDer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct SimConfig {
    http_base: String,
    quic_addr: SocketAddr,
    quic_server_name: String,
    quic_ca_cert: Option<PathBuf>,
    username: Option<String>,
    password: Option<String>,
    account_id: u64,
    auth_token: String,
    client_build: String,
    locale: String,
    timeout_ms: u64,
    skip_http: bool,
    skip_quic: bool,
    check_characters: bool,
    send_move_datagram: bool,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    success: bool,
    account_id: String,
    auth_token: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct CharacterListResponse {
    characters: Vec<CharacterInfo>,
}

#[derive(Debug, Deserialize)]
struct CharacterInfo {
    id: String,
    protocol_character_id: Option<u64>,
    name: String,
    level: u16,
    class: String,
}

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cfg = parse_args()?;

    println!("[sim-client] iniciando simulacao");
    println!("[sim-client] HTTP base: {}", cfg.http_base);
    println!(
        "[sim-client] QUIC addr: {} (server name: {})",
        cfg.quic_addr, cfg.quic_server_name
    );

    let mut login_account_hex: Option<String> = None;
    let mut login_auth_token: Option<String> = None;

    if !cfg.skip_http {
        let (account_hex, auth_token) = run_http_login_flow(&cfg).await?;
        login_account_hex = Some(account_hex.clone());
        login_auth_token = Some(auth_token);

        if cfg.account_id == 0 {
            cfg.account_id = derive_account_id_hint(&account_hex);
            println!(
                "[sim-client] account_id para protocolo derivado do login: {}",
                cfg.account_id
            );
        }
    }

    if !cfg.skip_quic {
        if cfg.auth_token.is_empty() {
            cfg.auth_token = login_auth_token
                .clone()
                .or(login_account_hex.clone())
                .unwrap_or_else(|| "sim-token".to_string());
        }

        run_quic_protocol_flow(&cfg).await?;
    }

    println!("[sim-client] simulacao concluida com sucesso");
    Ok(())
}

async fn run_http_login_flow(cfg: &SimConfig) -> anyhow::Result<(String, String)> {
    let username = cfg
        .username
        .as_deref()
        .ok_or_else(|| anyhow!("--username obrigatorio quando HTTP estiver habilitado"))?;
    let password = cfg
        .password
        .as_deref()
        .ok_or_else(|| anyhow!("--password obrigatorio quando HTTP estiver habilitado"))?;

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_millis(cfg.timeout_ms))
        .build()
        .context("falha ao criar client HTTP")?;

    let login_url = format!("{}/login", cfg.http_base.trim_end_matches('/'));
    println!("[sim-client] POST {}", login_url);

    let response = client
        .post(login_url)
        .json(&LoginRequest { username, password })
        .send()
        .await
        .context("falha de rede ao chamar /login")?;

    if response.status() != StatusCode::OK {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<sem body>".to_string());
        bail!("/login falhou com status {}: {}", status, body);
    }

    let login: LoginResponse = response
        .json()
        .await
        .context("falha ao decodificar resposta de /login")?;

    if !login.success {
        bail!("/login retornou success=false: {}", login.message);
    }

    println!(
        "[sim-client] login OK: account_id={} mensagem='{}'",
        login.account_id, login.message
    );

    if cfg.check_characters {
        let characters_url = format!("{}/characters", cfg.http_base.trim_end_matches('/'));
        println!("[sim-client] GET {}", characters_url);

        let chars_resp = client
            .get(characters_url)
            .send()
            .await
            .context("falha de rede ao chamar /characters")?;

        if chars_resp.status() != StatusCode::OK {
            let status = chars_resp.status();
            let body = chars_resp
                .text()
                .await
                .unwrap_or_else(|_| "<sem body>".to_string());
            bail!("/characters falhou com status {}: {}", status, body);
        }

        let payload: CharacterListResponse = chars_resp
            .json()
            .await
            .context("falha ao decodificar resposta de /characters")?;

        println!(
            "[sim-client] sessao autenticada OK: {} personagens",
            payload.characters.len()
        );

        for c in payload.characters.iter().take(3) {
            println!(
                "[sim-client] - {} ({}) lvl {} id={} protocol_id={} ",
                c.name,
                c.class,
                c.level,
                c.id,
                c.protocol_character_id.unwrap_or_default()
            );
        }
    }

    Ok((login.account_id, login.auth_token))
}

async fn run_quic_protocol_flow(cfg: &SimConfig) -> anyhow::Result<()> {
    let cert_path = cfg.quic_ca_cert.as_ref().ok_or_else(|| {
        anyhow!("--quic-ca-cert obrigatorio quando QUIC estiver habilitado (use --skip-quic para ignorar)")
    })?;

    let codec = WireCodec::default();
    let client_config = build_quic_client_config(cert_path)?;

    let bind_addr: SocketAddr = if cfg.quic_addr.is_ipv4() {
        "0.0.0.0:0".parse().expect("valid ipv4 bind")
    } else {
        "[::]:0".parse().expect("valid ipv6 bind")
    };

    let mut endpoint =
        Endpoint::client(bind_addr).context("falha ao criar endpoint QUIC client")?;
    endpoint.set_default_client_config(client_config);

    println!("[sim-client] conectando QUIC em {}", cfg.quic_addr);
    let connection = endpoint
        .connect(cfg.quic_addr, &cfg.quic_server_name)
        .context("falha ao iniciar handshake QUIC")?
        .await
        .context("falha ao completar handshake QUIC")?;

    println!("[sim-client] QUIC conectado");

    let hello_packet = WirePacket::client(
        1,
        RouteKey::LOBBY,
        1,
        None,
        now_ms(),
        ClientMessage::Hello(ClientHello {
            account_id: cfg.account_id,
            auth_token: cfg.auth_token.clone(),
            client_build: cfg.client_build.clone(),
            locale: cfg.locale.clone(),
        }),
    );

    let hello_frames = send_control_request(&connection, &codec, &hello_packet).await?;
    let hello_ack = hello_frames
        .iter()
        .find_map(|packet| match &packet.payload {
            PacketPayload::Server(ServerMessage::HelloAck {
                session_id,
                heartbeat_interval_ms,
                motd,
                characters,
            }) => Some((
                *session_id,
                *heartbeat_interval_ms,
                motd.clone(),
                characters.clone(),
            )),
            _ => None,
        })
        .ok_or_else(|| anyhow!("HelloAck nao recebido"))?;
    let hello_characters = hello_ack.3;
    let selected_character_id = hello_characters
        .first()
        .map(|entry| entry.character_id)
        .ok_or_else(|| anyhow!("HelloAck sem personagens; crie personagem na conta"))?;
    println!("[sim-client] protocolo OK: HelloAck recebido");
    println!(
        "[sim-client] handshake: session_id={} heartbeat={}ms motd='{}' personagens={} selecionado={}",
        hello_ack.0,
        hello_ack.1,
        hello_ack.2,
        hello_characters.len(),
        selected_character_id
    );

    let select_character_packet = WirePacket::client(
        1,
        RouteKey::LOBBY,
        2,
        Some(1),
        now_ms(),
        ClientMessage::SelectCharacter {
            character_id: selected_character_id,
        },
    );

    let transfer_frames =
        send_control_request(&connection, &codec, &select_character_packet).await?;
    let transfer = transfer_frames
        .iter()
        .find_map(|packet| match &packet.payload {
            PacketPayload::Server(ServerMessage::MapTransfer(directive)) => Some(directive.clone()),
            _ => None,
        })
        .ok_or_else(|| anyhow!("MapTransfer nao recebido apos SelectCharacter"))?;

    println!(
        "[sim-client] transfer recebido: transfer_id={} route={:?} expires_at_ms={}",
        transfer.transfer_id, transfer.route, transfer.expires_at_ms
    );

    let transfer_ack_packet = WirePacket::client(
        1,
        RouteKey::LOBBY,
        3,
        Some(2),
        now_ms(),
        ClientMessage::MapTransferAck {
            transfer_id: transfer.transfer_id,
            route_token: transfer.route_token.clone(),
        },
    );

    let enter_frames = send_control_request(&connection, &codec, &transfer_ack_packet).await?;
    assert_server_message(&enter_frames, |msg| {
        matches!(msg, ServerMessage::EnterMap { .. })
    })?;
    println!("[sim-client] protocolo OK: EnterMap recebido");

    let keepalive_packet = WirePacket::client(
        1,
        transfer.route,
        4,
        Some(3),
        now_ms(),
        ClientMessage::KeepAlive {
            client_time_ms: now_ms(),
        },
    );

    let keepalive_frames = send_control_request(&connection, &codec, &keepalive_packet).await?;
    assert_server_message(&keepalive_frames, |msg| {
        matches!(msg, ServerMessage::Pong { .. })
    })?;
    println!("[sim-client] protocolo OK: Pong recebido");

    if cfg.send_move_datagram {
        let move_packet = WirePacket::client(
            1,
            transfer.route,
            5,
            Some(4),
            now_ms(),
            ClientMessage::Move(protocol::MoveInput {
                client_tick: 1,
                x: 125,
                y: 126,
                direction: 2,
                path: [1, 2, 3, 4, 5, 6, 7, 8],
            }),
        );

        let datagram = codec
            .encode_datagram_frame(QuicChannel::GameplayInput, &move_packet)
            .context("falha ao codificar datagrama de movimento")?;

        connection
            .send_datagram(datagram.into())
            .map_err(|e| anyhow!("falha ao enviar datagrama de movimento: {}", e))?;

        println!("[sim-client] datagrama de movimento enviado");

        let response_datagram = tokio::time::timeout(
            Duration::from_millis(cfg.timeout_ms),
            connection.read_datagram(),
        )
        .await
        .context("timeout aguardando datagrama de resposta do servidor")?
        .map_err(|e| anyhow!("falha ao ler datagrama de resposta: {}", e))?;

        let decoded = codec
            .decode_datagram_frame(response_datagram.as_ref())
            .context("falha ao decodificar datagrama de resposta")?;

        match decoded.packet.payload {
            PacketPayload::Server(ServerMessage::StateDelta { entities, .. }) => {
                println!(
                    "[sim-client] protocolo OK: StateDelta recebido ({} entidades)",
                    entities.len()
                );
            }
            PacketPayload::Server(other) => {
                bail!(
                    "resposta inesperada ao Move: esperado StateDelta, recebido {:?}",
                    other
                );
            }
            PacketPayload::Client(_) => {
                bail!("resposta invalida ao Move: payload client");
            }
        }
    }

    connection.close(0u32.into(), b"sim-client done");
    endpoint.wait_idle().await;
    println!("[sim-client] conexao QUIC encerrada");
    Ok(())
}

async fn send_control_request(
    connection: &quinn::Connection,
    codec: &WireCodec,
    packet: &WirePacket,
) -> anyhow::Result<Vec<WirePacket>> {
    let frame = codec
        .encode_stream_frame(QuicChannel::Control, packet)
        .context("falha ao codificar frame de controle")?;

    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("falha ao abrir stream bidi")?;

    send.write_all(&frame)
        .await
        .context("falha ao enviar frame no stream")?;
    send.finish()
        .map_err(|e| anyhow!("falha ao finalizar envio do stream: {}", e))?;

    let bytes = recv
        .read_to_end(codec.limits().max_stream_payload_size.saturating_mul(8))
        .await
        .context("falha ao ler resposta do stream")?;

    decode_stream_frames(codec, &bytes)
}

fn decode_stream_frames(codec: &WireCodec, bytes: &[u8]) -> anyhow::Result<Vec<WirePacket>> {
    let mut consumed = 0usize;
    let mut packets = Vec::new();

    while consumed < bytes.len() {
        let slice = &bytes[consumed..];
        match codec.try_decode_stream_frame(slice)? {
            Some((decoded, used)) => {
                packets.push(decoded.packet);
                consumed += used;
            }
            None => {
                bail!(
                    "resposta de stream incompleta: consumed={} total={}",
                    consumed,
                    bytes.len()
                );
            }
        }
    }

    Ok(packets)
}

fn assert_server_message<F>(packets: &[WirePacket], predicate: F) -> anyhow::Result<()>
where
    F: Fn(&ServerMessage) -> bool,
{
    for packet in packets {
        if let PacketPayload::Server(message) = &packet.payload {
            if predicate(message) {
                return Ok(());
            }
        }
    }

    bail!("nao encontrou mensagem esperada nas respostas do servidor")
}

fn build_quic_client_config(ca_cert_path: &PathBuf) -> anyhow::Result<quinn::ClientConfig> {
    let certs = load_ca_certificates(ca_cert_path)?;

    let mut roots = rustls::RootCertStore::empty();
    for cert in certs {
        roots
            .add(cert)
            .context("falha ao adicionar certificado CA ao trust store")?;
    }

    let rustls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let quic_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(rustls_config)
        .map_err(|e| anyhow!("falha ao configurar crypto QUIC client: {}", e))?;

    Ok(quinn::ClientConfig::new(Arc::new(quic_crypto)))
}

fn load_ca_certificates(path: &PathBuf) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("falha ao ler certificado CA: {}", path.display()))?;

    let mut reader = Cursor::new(bytes);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .context("falha ao parsear PEM do certificado CA")?;

    if certs.is_empty() {
        bail!("arquivo de certificado CA vazio: {}", path.display());
    }

    Ok(certs)
}

fn parse_args() -> anyhow::Result<SimConfig> {
    let mut cfg = SimConfig {
        http_base: "http://127.0.0.1:8080".to_string(),
        quic_addr: SocketAddr::from_str("127.0.0.1:6000").expect("valid default QUIC addr"),
        quic_server_name: "localhost".to_string(),
        quic_ca_cert: None,
        username: None,
        password: None,
        account_id: 0,
        auth_token: String::new(),
        client_build: "sim-client/1.0".to_string(),
        locale: "pt-BR".to_string(),
        timeout_ms: 5_000,
        skip_http: false,
        skip_quic: false,
        check_characters: true,
        send_move_datagram: true,
    };

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--http-base" => cfg.http_base = next_arg_value(&mut args, &arg)?,
            "--quic-addr" => {
                let value = next_arg_value(&mut args, &arg)?;
                cfg.quic_addr = SocketAddr::from_str(&value)
                    .with_context(|| format!("--quic-addr invalido: {}", value))?;
            }
            "--quic-server-name" => cfg.quic_server_name = next_arg_value(&mut args, &arg)?,
            "--quic-ca-cert" => {
                cfg.quic_ca_cert = Some(PathBuf::from(next_arg_value(&mut args, &arg)?))
            }
            "--username" => cfg.username = Some(next_arg_value(&mut args, &arg)?),
            "--password" => cfg.password = Some(next_arg_value(&mut args, &arg)?),
            "--account-id" => {
                let value = next_arg_value(&mut args, &arg)?;
                cfg.account_id = value
                    .parse::<u64>()
                    .with_context(|| format!("--account-id invalido: {}", value))?;
            }
            "--auth-token" => cfg.auth_token = next_arg_value(&mut args, &arg)?,
            "--client-build" => cfg.client_build = next_arg_value(&mut args, &arg)?,
            "--locale" => cfg.locale = next_arg_value(&mut args, &arg)?,
            "--timeout-ms" => {
                let value = next_arg_value(&mut args, &arg)?;
                cfg.timeout_ms = value
                    .parse::<u64>()
                    .with_context(|| format!("--timeout-ms invalido: {}", value))?;
            }
            "--skip-http" => cfg.skip_http = true,
            "--skip-quic" => cfg.skip_quic = true,
            "--no-check-characters" => cfg.check_characters = false,
            "--no-move-datagram" => cfg.send_move_datagram = false,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                bail!(
                    "argumento desconhecido: {}\nUse --help para ver as opcoes.",
                    other
                );
            }
        }
    }

    if cfg.skip_http && cfg.skip_quic {
        bail!("simulador sem acoes: remova --skip-http ou --skip-quic");
    }

    if !cfg.skip_http {
        if cfg.username.is_none() || cfg.password.is_none() {
            bail!("--username e --password sao obrigatorios quando HTTP estiver habilitado");
        }
    }

    if !cfg.skip_quic && cfg.quic_ca_cert.is_none() {
        bail!("--quic-ca-cert e obrigatorio quando QUIC estiver habilitado");
    }

    Ok(cfg)
}

fn next_arg_value<I>(args: &mut std::iter::Peekable<I>, flag: &str) -> anyhow::Result<String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| anyhow!("valor ausente para {}", flag))
}

fn derive_account_id_hint(account_hex: &str) -> u64 {
    if account_hex.len() < 16 {
        return 0;
    }

    u64::from_str_radix(&account_hex[0..16], 16).unwrap_or(0)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn print_help() {
    println!(
        "sim-client - simulador de cliente para validar HTTP login + protocolo QUIC\n\n\
Uso:\n\
  cargo run --manifest-path server/Cargo.toml --bin sim-client -- [opcoes]\n\n\
Opcoes:\n\
  --http-base <url>            Base HTTP (default: http://127.0.0.1:8080)\n\
  --username <user>            Usuario para /login\n\
  --password <pass>            Senha para /login\n\
  --no-check-characters        Nao chamar /characters apos login\n\
  --quic-addr <ip:port>        Endereco QUIC (default: 127.0.0.1:6000)\n\
  --quic-server-name <name>    SNI/ServerName QUIC (default: localhost)\n\
  --quic-ca-cert <pem>         Certificado CA/servidor para validar TLS QUIC\n\
  --account-id <u64>           account_id enviado no ClientHello (default: derivado do login)\n\
  --auth-token <token>         auth_token enviado no ClientHello\n\
  --client-build <str>         client_build enviado no ClientHello\n\
  --locale <str>               locale enviado no ClientHello\n\
  --no-move-datagram           Nao envia datagrama de Move apos handshake\n\
  --skip-http                  Pula validacao HTTP\n\
  --skip-quic                  Pula validacao QUIC\n\
  --timeout-ms <ms>            Timeout HTTP (default: 5000)\n\
  --help                       Mostra esta ajuda\n"
    );
}
