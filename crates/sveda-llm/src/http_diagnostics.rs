use std::error::Error;
use std::time::Instant;

use reqwest::header::{HeaderMap, CONTENT_ENCODING, CONTENT_TYPE, TRANSFER_ENCODING};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use sveda_protocol::diagnostics::{identifier, log, Level};

use crate::LlmError;

pub fn media_type(headers: &HeaderMap) -> &str {
    headers
        .get(CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or_default().trim())
        .unwrap_or_default()
}

pub fn content_encoding(headers: &HeaderMap) -> &'static str {
    header_kind(
        headers,
        CONTENT_ENCODING,
        &["identity", "gzip", "br", "deflate", "zstd"],
    )
}

fn header_kind(
    headers: &HeaderMap,
    name: reqwest::header::HeaderName,
    known: &[&'static str],
) -> &'static str {
    let mut values = headers.get_all(name).iter();
    let Some(header) = values.next() else {
        return "";
    };
    if values.next().is_some() {
        return "multiple";
    }
    known
        .iter()
        .find(|value| header.as_bytes().eq_ignore_ascii_case(value.as_bytes()))
        .copied()
        .unwrap_or("other")
}

pub fn response_fields(response: &reqwest::Response, key: &str, started: Instant) -> Value {
    let headers = response.headers();
    let media = media_type(headers);
    let media = [
        "",
        "text/event-stream",
        "application/json",
        "text/html",
        "text/plain",
        "application/octet-stream",
    ]
    .into_iter()
    .find(|value| media.eq_ignore_ascii_case(value))
    .unwrap_or("other");
    let request_ids: Map<String, Value> = [
        "x-request-id",
        "request-id",
        "x-ds-request-id",
        "x-deepseek-request-id",
        "cf-ray",
    ]
    .into_iter()
    .filter_map(|name| {
        let header = headers.get(name)?;
        let value = header.to_str().ok().unwrap_or_default();
        // Keep common provider correlation IDs. Hash other arbitrary header bytes.
        let conventional = ["req_", "req-", "request_"]
            .iter()
            .any(|prefix| value.starts_with(prefix))
            || (value.len() == 36 && value.as_bytes()[8] == b'-')
            || (name == "cf-ray" && value.len() == 20);
        let safe = identifier(value);
        let id = if conventional && value == safe && !value.contains(key) {
            safe.to_owned()
        } else {
            format!("sha256:{:x}", Sha256::digest(header.as_bytes()))
        };
        Some((name.to_owned(), json!(id)))
    })
    .collect();
    json!({
        "status": response.status().as_u16(),
        "http_version": format!("{:?}", response.version()),
        "content_type": media,
        "content_encoding": content_encoding(headers),
        "transfer_encoding": header_kind(headers, TRANSFER_ENCODING, &["chunked", "identity"]),
        "content_length": response.content_length(),
        "request_ids": request_ids,
        "headers_ms": started.elapsed().as_millis(),
    })
}

pub fn read_error(
    error: &reqwest::Error,
    phase: &'static str,
    stats: impl FnOnce() -> Value,
) -> LlmError {
    log(Level::Warn, "llm.http.transport_error", || {
        let mut chain = Vec::new();
        let mut source = error.source();
        while let Some(cause) = source {
            // Hyper's Display is a fixed library diagnostic. Nested causes may
            // contain addresses, paths or payloads, so only log safe attributes.
            let entry = if let Some(error) = cause.downcast_ref::<reqwest::Error>() {
                json!({
                    "type": "reqwest", "is_decode": error.is_decode(),
                    "is_timeout": error.is_timeout(), "is_body": error.is_body(),
                    "is_connect": error.is_connect(), "is_request": error.is_request(),
                })
            } else if let Some(error) = cause.downcast_ref::<hyper::Error>() {
                json!({"type": "hyper", "kind": error.to_string()})
            } else if let Some(error) = cause.downcast_ref::<std::io::Error>() {
                json!({"type": "io", "kind": format!("{:?}", error.kind()), "os_code": error.raw_os_error()})
            } else {
                let kind = match cause.to_string().as_str() {
                    "operation timed out" | "request timed out" => "timeout",
                    "end of file before message length reached" => "premature_eof",
                    "unexpected EOF during chunk size line" => "chunk_header_eof",
                    _ => "redacted",
                };
                json!({"type": kind})
            };
            chain.push(entry);
            source = cause.source();
        }
        json!({
            "phase": phase,
            "is_decode": error.is_decode(),
            "is_timeout": error.is_timeout(),
            "is_body": error.is_body(),
            "is_connect": error.is_connect(),
            "is_request": error.is_request(),
            "chain": chain,
            "stream": stats(),
        })
    });
    let message = if error.is_timeout() {
        "The model provider timed out."
    } else if phase == "headers" {
        "The model provider could not be reached."
    } else {
        "The connection to the model provider ended while reading its response."
    };
    LlmError::Other(message.into())
}

// A bounded structural sample distinguishes JSON, HTML and binary data
// without exposing any text or credentials in upstream payloads.
pub fn redacted_shape(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(96)
        .map(|byte| match byte {
            b'{' | b'}' | b'[' | b']' | b':' | b',' | b'"' | b'<' | b'>' | b' ' | b'\r' | b'\n' => {
                char::from(*byte)
            }
            b'!'..=b'~' => '*',
            _ => '?',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::redacted_shape;

    #[test]
    fn body_sample_keeps_only_bounded_structure() {
        let sample = redacted_shape(
            br#"{"Authorization":"Bearer fake-token","cookie":"PHPSESSID=fake-secret"}"#,
        );
        assert_eq!(
            sample,
            r#"{"*************":"****** **********","******":"*********************"}"#
        );
        assert_eq!(redacted_shape(&[b'x'; 200]).len(), 96);
    }
}
