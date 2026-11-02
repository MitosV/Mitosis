use std::time::{SystemTime, UNIX_EPOCH};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub const MITOSIS_DEFAULT_NAMESPACE: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"; // Default DNS namespace base or Mitosis seed

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("Invalid token format")]
    InvalidTokenFormat,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token signature")]
    InvalidSignature,
    #[error("UUID parse error: {0}")]
    UuidParse(#[from] uuid::Error),
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),
}

#[derive(Debug, Clone)]
pub struct IdentityManager {
    namespace: Uuid,
    secret_key: Vec<u8>,
}

impl IdentityManager {
    pub fn new(namespace: Uuid, secret_key: impl Into<Vec<u8>>) -> Self {
        Self {
            namespace,
            secret_key: secret_key.into(),
        }
    }

    pub fn with_default_namespace(secret_key: impl Into<Vec<u8>>) -> Self {
        let ns = Uuid::parse_str(MITOSIS_DEFAULT_NAMESPACE).unwrap();
        Self::new(ns, secret_key)
    }

    /// Generates persistent, deterministic UUID v5 from an immutable internal ID.
    /// Example internal_id: "group/afk/bot/1"
    pub fn generate_uuid(&self, internal_id: &str) -> Uuid {
        Uuid::new_v5(&self.namespace, internal_id.as_bytes())
    }

    /// Helper to format internal identifier: "group/{group_name}/bot/{bot_index}"
    pub fn make_internal_id(group: &str, bot_index: usize) -> String {
        format!("group/{}/bot/{}", group, bot_index)
    }

    /// Generates a signed, short-lived session authorization token for Mitosis Bridge.
    /// Format before Base64: "{timestamp_ms}:{uuid}:{username}:{hmac_hex}"
    pub fn generate_auth_token(&self, uuid: &Uuid, username: &str, valid_secs: u64) -> String {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let expires_at_ms = now_ms + (valid_secs * 1000);
        let payload = format!("{}:{}:{}", expires_at_ms, uuid, username);

        let mut mac = HmacSha256::new_from_slice(&self.secret_key).expect("Valid HMAC key");
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let full_token = format!("{}:{}", payload, signature);
        BASE64.encode(full_token.as_bytes())
    }

    /// Verifies a session token, returning (UUID, username) if valid and not expired.
    pub fn verify_auth_token(&self, token_b64: &str) -> Result<(Uuid, String), IdentityError> {
        let decoded = BASE64.decode(token_b64.as_bytes())?;
        let token_str = String::from_utf8(decoded).map_err(|_| IdentityError::InvalidTokenFormat)?;

        let parts: Vec<&str> = token_str.split(':').collect();
        if parts.len() != 4 {
            return Err(IdentityError::InvalidTokenFormat);
        }

        let expires_at_ms: u64 = parts[0].parse().map_err(|_| IdentityError::InvalidTokenFormat)?;
        let uuid = Uuid::parse_str(parts[1])?;
        let username = parts[2].to_string();
        let provided_sig = parts[3];

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if now_ms > expires_at_ms {
            return Err(IdentityError::TokenExpired);
        }

        let payload = format!("{}:{}:{}", parts[0], parts[1], parts[2]);
        let mut mac = HmacSha256::new_from_slice(&self.secret_key).expect("Valid HMAC key");
        mac.update(payload.as_bytes());
        let expected_sig = hex::encode(mac.finalize().into_bytes());

        if provided_sig != expected_sig {
            return Err(IdentityError::InvalidSignature);
        }

        Ok((uuid, username))
    }
}

// Simple hex helper to avoid extra dependency
mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        let mut s = String::with_capacity(data.as_ref().len() * 2);
        for &b in data.as_ref() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v5_determinism() {
        let mgr = IdentityManager::with_default_namespace("secret_test_key");
        let id1 = mgr.generate_uuid("group/afk/bot/1");
        let id2 = mgr.generate_uuid("group/afk/bot/1");
        let id3 = mgr.generate_uuid("group/afk/bot/2");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(id1.get_version_num(), 5);
    }

    #[test]
    fn test_auth_token_verification() {
        let mgr = IdentityManager::with_default_namespace("secret_test_key");
        let uuid = mgr.generate_uuid("group/minigame/bot/1");
        let token = mgr.generate_auth_token(&uuid, "botgame_1", 60);

        let (verified_uuid, verified_name) = mgr.verify_auth_token(&token).unwrap();
        assert_eq!(verified_uuid, uuid);
        assert_eq!(verified_name, "botgame_1");
    }

    #[test]
    fn test_invalid_signature_fails() {
        let mgr1 = IdentityManager::with_default_namespace("key1");
        let mgr2 = IdentityManager::with_default_namespace("key2");
        let uuid = mgr1.generate_uuid("bot/1");
        let token = mgr1.generate_auth_token(&uuid, "bot1", 60);

        assert!(mgr2.verify_auth_token(&token).is_err());
    }
}
