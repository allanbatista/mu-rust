use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use mongodb::bson::oid::ObjectId;
use protocol::message::CharacterSummary;
use protocol::RouteKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const MIN_SECRET_LEN: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum AuthTokenError {
    #[error("auth token secret is too short (min {MIN_SECRET_LEN} bytes)")]
    SecretTooShort,

    #[error("invalid auth token format")]
    InvalidFormat,

    #[error("auth token signature is invalid")]
    InvalidSignature,

    #[error("auth token is expired")]
    Expired,

    #[error("failed to decode auth token payload")]
    PayloadDecode,

    #[error("failed to parse auth token payload")]
    PayloadParse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthCharacterSummary {
    pub character_id: u64,
    pub db_id: String,
    pub name: String,
    pub class_id: u8,
    pub level: u16,
}

impl AuthCharacterSummary {
    pub fn into_protocol(self) -> CharacterSummary {
        CharacterSummary {
            character_id: self.character_id,
            name: self.name,
            class_id: self.class_id,
            level: self.level,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthSessionClaims {
    pub account_id: u64,
    pub session_id: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub characters: Vec<AuthCharacterSummary>,
}

impl AuthSessionClaims {
    pub fn new(
        account_id: u64,
        session_id: String,
        issued_at_ms: u64,
        expires_at_ms: u64,
        characters: Vec<AuthCharacterSummary>,
    ) -> Self {
        Self {
            account_id,
            session_id,
            issued_at_ms,
            expires_at_ms,
            characters,
        }
    }

    pub fn is_expired(&self, reference_ms: u64) -> bool {
        reference_ms >= self.expires_at_ms
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapTransferTokenClaims {
    pub session_id: u64,
    pub transfer_id: u64,
    pub character_id: u64,
    pub route: RouteKey,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

impl MapTransferTokenClaims {
    pub fn is_expired(&self, reference_ms: u64) -> bool {
        reference_ms >= self.expires_at_ms
    }
}

#[derive(Clone)]
pub struct AuthTokenService {
    secret: Arc<[u8]>,
    ttl: Duration,
}

impl AuthTokenService {
    pub fn new(secret: Vec<u8>, ttl: Duration) -> Result<Self, AuthTokenError> {
        if secret.len() < MIN_SECRET_LEN {
            return Err(AuthTokenError::SecretTooShort);
        }

        Ok(Self {
            secret: Arc::<[u8]>::from(secret),
            ttl,
        })
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn issue_session_token(
        &self,
        account_id: u64,
        session_id: String,
        characters: Vec<AuthCharacterSummary>,
        issued_at_ms: u64,
    ) -> Result<String, AuthTokenError> {
        let expires_at_ms = issued_at_ms.saturating_add(self.ttl.as_millis() as u64);
        let claims = AuthSessionClaims::new(
            account_id,
            session_id,
            issued_at_ms,
            expires_at_ms,
            characters,
        );
        self.issue(&claims)
    }

    pub fn issue(&self, claims: &AuthSessionClaims) -> Result<String, AuthTokenError> {
        self.issue_payload(claims)
    }

    pub fn issue_transfer_token(
        &self,
        claims: &MapTransferTokenClaims,
    ) -> Result<String, AuthTokenError> {
        self.issue_payload(claims)
    }

    pub fn verify(
        &self,
        token: &str,
        reference_ms: u64,
    ) -> Result<AuthSessionClaims, AuthTokenError> {
        let claims: AuthSessionClaims = self.verify_payload(token)?;

        if claims.session_id.is_empty() || claims.is_expired(reference_ms) {
            return Err(AuthTokenError::Expired);
        }

        Ok(claims)
    }

    pub fn verify_transfer_token(
        &self,
        token: &str,
        reference_ms: u64,
    ) -> Result<MapTransferTokenClaims, AuthTokenError> {
        let claims: MapTransferTokenClaims = self.verify_payload(token)?;
        if claims.is_expired(reference_ms) {
            return Err(AuthTokenError::Expired);
        }

        Ok(claims)
    }

    fn issue_payload<T: Serialize>(&self, payload: &T) -> Result<String, AuthTokenError> {
        let bytes = serde_json::to_vec(payload).map_err(|_| AuthTokenError::PayloadParse)?;
        let payload_b64 = URL_SAFE_NO_PAD.encode(bytes);
        let signature = self.sign(payload_b64.as_bytes())?;
        let signature_b64 = URL_SAFE_NO_PAD.encode(signature);
        Ok(format!("{payload_b64}.{signature_b64}"))
    }

    fn verify_payload<T: for<'de> Deserialize<'de>>(
        &self,
        token: &str,
    ) -> Result<T, AuthTokenError> {
        let (payload_b64, signature_b64) =
            token.split_once('.').ok_or(AuthTokenError::InvalidFormat)?;

        let signature = URL_SAFE_NO_PAD
            .decode(signature_b64)
            .map_err(|_| AuthTokenError::InvalidFormat)?;

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| AuthTokenError::InvalidSignature)?;
        mac.update(payload_b64.as_bytes());
        mac.verify_slice(&signature)
            .map_err(|_| AuthTokenError::InvalidSignature)?;

        let payload = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| AuthTokenError::PayloadDecode)?;

        serde_json::from_slice(&payload).map_err(|_| AuthTokenError::PayloadParse)
    }

    fn sign(&self, bytes: &[u8]) -> Result<Vec<u8>, AuthTokenError> {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| AuthTokenError::InvalidSignature)?;
        mac.update(bytes);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn object_id_to_u64(id: &ObjectId) -> u64 {
    object_id_hex_to_u64(&id.to_hex())
}

pub fn object_id_hex_to_u64(hex: &str) -> u64 {
    if hex.len() < 16 {
        return 0;
    }
    u64::from_str_radix(&hex[..16], 16).unwrap_or(0)
}

pub fn class_name_to_id(raw: &str) -> u8 {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "wizard" | "darkwizard" | "soulmaster" | "grandmaster" => 0,
        "knight" | "darkknight" | "bladeknight" | "blademaster" => 1,
        "elf" | "fairyelf" | "museelf" | "highelf" => 2,
        "magicgladiator" | "duelmaster" => 3,
        "darklord" | "lordemperor" => 4,
        "summoner" | "bloodysummoner" | "dimensionmaster" => 5,
        "ragefighter" | "fistmaster" => 6,
        _ => 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> AuthTokenService {
        AuthTokenService::new(
            b"01234567890123456789012345678901".to_vec(),
            Duration::from_secs(30),
        )
        .expect("valid service")
    }

    #[test]
    fn issue_and_verify_roundtrip() {
        let service = test_service();
        let token = service
            .issue_session_token(
                77,
                "session-1".to_string(),
                vec![AuthCharacterSummary {
                    character_id: 10,
                    db_id: "507f1f77bcf86cd799439011".to_string(),
                    name: "Knight".to_string(),
                    class_id: 1,
                    level: 150,
                }],
                1_000,
            )
            .expect("issue token");

        let claims = service.verify(&token, 1_500).expect("verify token");
        assert_eq!(claims.account_id, 77);
        assert_eq!(claims.session_id, "session-1");
        assert_eq!(claims.characters.len(), 1);
    }

    #[test]
    fn rejects_tampered_token() {
        let service = test_service();
        let token = service
            .issue_session_token(1, "s".to_string(), Vec::new(), 10)
            .expect("issue token");
        let (payload, signature) = token.split_once('.').expect("token split");
        let mut chars: Vec<char> = payload.chars().collect();
        chars[0] = if chars[0] == 'A' { 'B' } else { 'A' };
        let tampered_payload: String = chars.into_iter().collect();
        let tampered = format!("{tampered_payload}.{signature}");

        assert!(matches!(
            service.verify(&tampered, 20),
            Err(AuthTokenError::InvalidSignature)
        ));
    }

    #[test]
    fn rejects_expired_token() {
        let service = test_service();
        let token = service
            .issue_session_token(1, "s".to_string(), Vec::new(), 1_000)
            .expect("issue token");

        assert!(matches!(
            service.verify(&token, 35_000),
            Err(AuthTokenError::Expired)
        ));
    }

    #[test]
    fn transfer_token_roundtrip_and_expiration() {
        let service = test_service();
        let claims = MapTransferTokenClaims {
            session_id: 7,
            transfer_id: 100,
            character_id: 55,
            route: RouteKey {
                world_id: 1,
                entry_id: 1,
                map_id: 0,
                instance_id: 1,
            },
            issued_at_ms: 1_000,
            expires_at_ms: 2_000,
        };

        let token = service
            .issue_transfer_token(&claims)
            .expect("issue transfer token");
        let parsed = service
            .verify_transfer_token(&token, 1_500)
            .expect("verify transfer token");
        assert_eq!(parsed, claims);

        assert!(matches!(
            service.verify_transfer_token(&token, 2_500),
            Err(AuthTokenError::Expired)
        ));
    }
}
