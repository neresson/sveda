use hmac::{Hmac, Mac};
use sha2::Sha256;
use veda_protocol::TOKEN_PREFIX;

use crate::now_unix;

type HmacSha256 = Hmac<Sha256>;

pub fn issue(hmac_key: &[u8], visitor_id: &str, ttl_seconds: u64) -> String {
    let expires_at = now_unix().saturating_add(ttl_seconds.max(60));
    let payload = serde_json::json!({
        "visitor_id": visitor_id,
        "expires_at": expires_at,
    });
    let encoded = encode_payload(&payload.to_string());
    format!("{TOKEN_PREFIX}{encoded}.{}", sign(hmac_key, &encoded))
}

pub fn validate(hmac_key: &[u8], token: &str) -> Option<String> {
    let body = token.strip_prefix(TOKEN_PREFIX)?;
    let (encoded, signature) = body.rsplit_once('.')?;
    if !keys_match(&sign(hmac_key, encoded), signature) {
        return None;
    }

    let decoded = decode_payload(encoded)?;
    let payload: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let visitor_id = payload.get("visitor_id")?.as_str()?.trim();
    let expires_at = payload.get("expires_at")?.as_u64()?;
    if visitor_id.is_empty() || visitor_id.len() > 64 || expires_at < now_unix() {
        return None;
    }

    Some(visitor_id.to_string())
}

fn encode_payload(json: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json.as_bytes())
}

fn decode_payload(encoded: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
}

pub(crate) fn sign(hmac_key: &[u8], encoded: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(hmac_key).expect("hmac key");
    mac.update(encoded.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn keys_match(expected: &str, provided: &str) -> bool {
    if expected.len() != provided.len() {
        return false;
    }
    expected
        .bytes()
        .zip(provided.bytes())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_token_round_trips() {
        let token = issue(b"secret", "visitor-1", 120);
        assert!(token.starts_with(TOKEN_PREFIX));
        assert_eq!(validate(b"secret", &token).as_deref(), Some("visitor-1"));
        assert!(validate(b"other", &token).is_none());
    }
}
