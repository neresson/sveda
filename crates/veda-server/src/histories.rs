use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::{visitor_from, AppState};

#[derive(Deserialize)]
pub struct PatchHistory {
    pub title: String,
}

pub async fn list_histories(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let visitor_id = match visitor_from(&state, &headers) {
        Ok(visitor_id) => visitor_id,
        Err(status) => return status.into_response(),
    };
    Json(serde_json::json!({ "histories": state.store.list(&visitor_id) })).into_response()
}

pub async fn show_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chat_id): Path<String>,
) -> Response {
    let visitor_id = match visitor_from(&state, &headers) {
        Ok(visitor_id) => visitor_id,
        Err(status) => return status.into_response(),
    };
    match state.store.get(&visitor_id, &chat_id) {
        Some(history) => Json(serde_json::json!({ "history": history })).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn update_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chat_id): Path<String>,
    Json(payload): Json<PatchHistory>,
) -> Response {
    let visitor_id = match visitor_from(&state, &headers) {
        Ok(visitor_id) => visitor_id,
        Err(status) => return status.into_response(),
    };
    let title = payload.title.trim();
    if title.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "message": "title is required" })),
        )
            .into_response();
    }
    if !state.store.rename(&visitor_id, &chat_id, title) {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(serde_json::json!({ "success": true })).into_response()
}

pub async fn delete_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chat_id): Path<String>,
) -> Response {
    let visitor_id = match visitor_from(&state, &headers) {
        Ok(visitor_id) => visitor_id,
        Err(status) => return status.into_response(),
    };
    if !state.store.delete(&visitor_id, &chat_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(serde_json::json!({ "success": true })).into_response()
}
