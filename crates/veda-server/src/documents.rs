use axum::extract::{Multipart, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use veda_protocol::ExtractItem;

use crate::{visitor_from, AppState};

pub async fn extract_documents(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    if let Err(status) = visitor_from(&state, &headers) {
        return status.into_response();
    }

    let max_files = state.config.document_max_files.max(1);
    let max_bytes = state
        .config
        .document_max_file_kb
        .max(1)
        .saturating_mul(1024);
    let mut remaining_chars = state.config.document_max_total_chars.max(1);
    let mut items = Vec::new();
    let mut seen = 0usize;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({ "message": error.to_string() })),
                )
                    .into_response();
            }
        };
        let name = field.name().unwrap_or("").to_string();
        if name != "files" && !name.starts_with("files") {
            continue;
        }
        let filename = field
            .file_name()
            .map(ToOwned::to_owned)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "file".into());
        seen += 1;
        if seen > max_files {
            items.push(ExtractItem {
                filename,
                ok: false,
                text: None,
                error: Some("too many files".into()),
            });
            continue;
        }
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                items.push(ExtractItem {
                    filename,
                    ok: false,
                    text: None,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };
        if bytes.len() > max_bytes {
            items.push(ExtractItem {
                filename,
                ok: false,
                text: None,
                error: Some("file too large".into()),
            });
            continue;
        }
        if bytes.contains(&0) {
            items.push(ExtractItem {
                filename,
                ok: false,
                text: None,
                error: Some("unsupported binary file".into()),
            });
            continue;
        }
        let Ok(mut text) = String::from_utf8(bytes.to_vec()) else {
            items.push(ExtractItem {
                filename,
                ok: false,
                text: None,
                error: Some("file is not valid utf-8".into()),
            });
            continue;
        };
        if text.chars().count() > remaining_chars {
            text = text.chars().take(remaining_chars).collect();
        }
        remaining_chars = remaining_chars.saturating_sub(text.chars().count());
        items.push(ExtractItem {
            filename,
            ok: true,
            text: Some(text),
            error: None,
        });
    }

    Json(serde_json::json!({ "items": items })).into_response()
}
