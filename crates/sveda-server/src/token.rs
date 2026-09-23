use hmac::{Hmac, Mac};
use sha2::Sha256;
use sveda_protocol::{ADMIN_VISITOR_ID, TOKEN_PREFIX};

use crate::now_unix;
use crate::policy::{claims_restricted, CapabilityPolicy};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenClaims {
    pub visitor_id: String,
    pub admin: bool,
    pub policy: Option<String>,
    pub grants: Option<CapabilityPolicy>,
}

#[derive(Debug, Clone, Default)]
pub struct IssueOptions {
    pub admin: bool,
    pub policy: Option<String>,
    pub grants: Option<CapabilityPolicy>,
}

pub fn issue(hmac_key: &[u8], visitor_id: &str, ttl_seconds: u64) -> String {
    issue_with_options(hmac_key, visitor_id, ttl_seconds, IssueOptions::default())
}

pub fn issue_admin(hmac_key: &[u8], ttl_seconds: u64) -> String {
    issue_with_options(
        hmac_key,
        ADMIN_VISITOR_ID,
        ttl_seconds,
        IssueOptions {
            admin: true,
            ..IssueOptions::default()
        },
    )
}

pub fn issue_with_options(
    hmac_key: &[u8],
    visitor_id: &str,
    ttl_seconds: u64,
    options: IssueOptions,
) -> String {
    let expires_at = now_unix().saturating_add(ttl_seconds.max(60));
    let mut payload = serde_json::json!({
        "visitor_id": visitor_id,
        "expires_at": expires_at,
    });
    if options.admin {
        payload["admin"] = serde_json::json!(true);
    }
    if let Some(policy) = options
        .policy
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        payload["policy"] = serde_json::json!(policy);
    }
    if let Some(grants) = options.grants {
        if let Ok(value) = serde_json::to_value(grants) {
            payload["grants"] = value;
        }
    }
    let encoded = encode_payload(&payload.to_string());
    format!("{TOKEN_PREFIX}{encoded}.{}", sign(hmac_key, &encoded))
}

pub fn validate(hmac_key: &[u8], token: &str) -> Option<String> {
    parse(hmac_key, token).map(|claims| claims.visitor_id)
}

pub fn parse(hmac_key: &[u8], token: &str) -> Option<TokenClaims> {
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
    let admin = payload
        .get("admin")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && visitor_id == ADMIN_VISITOR_ID;

    let policy = payload
        .get("policy")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let grants = payload
        .get("grants")
        .and_then(|value| serde_json::from_value::<CapabilityPolicy>(value.clone()).ok());

    if claims_restricted(policy.as_deref(), grants.as_ref()) && admin {
        return None;
    }

    Some(TokenClaims {
        visitor_id: visitor_id.to_string(),
        admin,
        policy,
        grants,
    })
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
    use crate::policy::CapabilityPolicy;

    #[test]
    fn issued_token_round_trips() {
        let token = issue(b"secret", "visitor-1", 120);
        assert!(token.starts_with(TOKEN_PREFIX));
        assert_eq!(validate(b"secret", &token).as_deref(), Some("visitor-1"));
        assert!(validate(b"other", &token).is_none());
        assert!(!parse(b"secret", &token).unwrap().admin);
    }

    #[test]
    fn admin_token_is_marked_admin() {
        let token = issue_admin(b"secret", 120);
        let claims = parse(b"secret", &token).unwrap();
        assert_eq!(claims.visitor_id, ADMIN_VISITOR_ID);
        assert!(claims.admin);
    }

    #[test]
    fn visitor_named_admin_without_claim_is_not_admin() {
        let token = issue(b"secret", ADMIN_VISITOR_ID, 120);
        let claims = parse(b"secret", &token).unwrap();
        assert!(!claims.admin);
    }

    fn signed(hmac_key: &[u8], payload: serde_json::Value) -> String {
        let encoded = encode_payload(&payload.to_string());
        format!("{TOKEN_PREFIX}{encoded}.{}", sign(hmac_key, &encoded))
    }

    #[test]
    fn admin_flag_on_a_public_visitor_is_ignored() {
        let token = signed(
            b"secret",
            serde_json::json!({
                "visitor_id": "visitor-1",
                "expires_at": now_unix() + 120,
                "admin": true
            }),
        );
        let claims = parse(b"secret", &token).unwrap();
        assert_eq!(claims.visitor_id, "visitor-1");
        assert!(!claims.admin);
    }

    #[test]
    fn admin_flag_requires_exact_reserved_visitor() {
        let token = signed(
            b"secret",
            serde_json::json!({
                "visitor_id": "SVEDA-ADMIN",
                "expires_at": now_unix() + 120,
                "admin": true
            }),
        );
        let claims = parse(b"secret", &token).unwrap();
        assert!(!claims.admin);
    }

    #[test]
    fn string_admin_flag_is_not_admin() {
        let token = signed(
            b"secret",
            serde_json::json!({
                "visitor_id": ADMIN_VISITOR_ID,
                "expires_at": now_unix() + 120,
                "admin": "true"
            }),
        );
        let claims = parse(b"secret", &token).unwrap();
        assert!(!claims.admin);
    }

    #[test]
    fn expired_token_is_rejected() {
        let token = signed(
            b"secret",
            serde_json::json!({
                "visitor_id": ADMIN_VISITOR_ID,
                "expires_at": 1,
                "admin": true
            }),
        );
        assert!(parse(b"secret", &token).is_none());
    }

    #[test]
    fn wrong_signature_is_rejected() {
        let token = issue_admin(b"secret", 120);
        let forged = format!("{token}00");
        assert!(parse(b"secret", &forged).is_none());
        assert!(parse(b"other", &token).is_none());
    }

    #[test]
    fn empty_or_oversized_visitor_is_rejected() {
        let too_long = "v".repeat(65);
        for visitor_id in ["", too_long.as_str()] {
            let token = signed(
                b"secret",
                serde_json::json!({
                    "visitor_id": visitor_id,
                    "expires_at": now_unix() + 120,
                    "admin": true
                }),
            );
            assert!(parse(b"secret", &token).is_none(), "{visitor_id:?}");
        }
    }

    #[test]
    fn validate_returns_admin_visitor_id() {
        let token = issue_admin(b"secret", 120);
        assert_eq!(
            validate(b"secret", &token).as_deref(),
            Some(ADMIN_VISITOR_ID)
        );
    }

    #[test]
    fn policy_and_grants_round_trip() {
        let token = issue_with_options(
            b"secret",
            "visitor-1",
            120,
            IssueOptions {
                policy: Some("reader".into()),
                grants: Some(CapabilityPolicy {
                    web: Some(false),
                    code: None,
                    mcp: None,
                    client: None,
                }),
                ..IssueOptions::default()
            },
        );
        let claims = parse(b"secret", &token).unwrap();
        assert_eq!(claims.policy.as_deref(), Some("reader"));
        assert_eq!(claims.grants.as_ref().and_then(|g| g.web), Some(false));
    }

    #[test]
    fn admin_token_rejects_policy_claims() {
        let token = issue_with_options(
            b"secret",
            ADMIN_VISITOR_ID,
            120,
            IssueOptions {
                admin: true,
                policy: Some("reader".into()),
                ..IssueOptions::default()
            },
        );
        assert!(parse(b"secret", &token).is_none());
    }
}
