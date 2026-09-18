use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::settings::{SettingsPatch, SettingsStore};
use crate::{keys_match, visitor_from, AppState};

pub fn expected_admin_key(state: &AppState) -> Option<String> {
    state
        .admin_key
        .lock()
        .expect("admin key")
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    if crate::ui::session_ok(state, headers) {
        return Ok(());
    }
    let Some(expected) = expected_admin_key(state) else {
        return Err(StatusCode::NOT_FOUND);
    };

    let provided = header_admin_key(headers);
    match provided {
        Some(value) if keys_match(&expected, &value) => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn header_admin_key(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get(sveda_protocol::HEADER_ADMIN_KEY)
        .and_then(|value| value.to_str().ok())
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty() && !token.starts_with(sveda_protocol::TOKEN_PREFIX))
        .map(ToOwned::to_owned)
}

pub async fn show_settings(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    Json(state.settings.document().masked()).into_response()
}

pub async fn update_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(patch): Json<SettingsPatch>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    let current = state.settings.document();
    let merged = current.preserve_secrets(current.merge(patch));
    let stored = match state.settings.persist(merged).await {
        Ok(stored) => stored,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    apply_runtime(&state, &stored);
    Json(stored.masked()).into_response()
}

pub async fn embed_config(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(status) = visitor_from(&state, &headers) {
        return status.into_response();
    }
    Json(state.settings.document().public()).into_response()
}

pub(crate) fn apply_runtime(state: &AppState, document: &crate::settings::SettingsDocument) {
    *state.catalog.lock().expect("catalog") = document.to_catalog();
    let mut origins = state.cors_origins.lock().expect("cors");
    *origins = document.cors.allowed_origins.clone();
    let security = document
        .security
        .clone()
        .unwrap_or_else(|| crate::settings::SecuritySettings::from_config(&state.config));
    state
        .occupancy
        .set_limits(security.occupancy_global, security.occupancy_per_visitor);
    state.throttle.set_limits(
        security.embed_token_throttle_max,
        Duration::from_secs(security.embed_token_throttle_window_secs.max(1)),
    );
    state.stream_throttle.set_limits(
        security.stream_throttle_max,
        Duration::from_secs(security.stream_throttle_window_secs.max(1)),
    );
    state.ip_throttle.set_limits(
        security.ip_throttle_max,
        Duration::from_secs(security.ip_throttle_window_secs.max(1)),
    );
    *state.client_ip_header.lock().expect("ip header") = security.client_ip_header;
}

pub fn overlay(base: &crate::Config, settings: &SettingsStore) -> crate::Config {
    let document = settings.document();
    let mut config = base.clone();
    config.max_steps = document.max_steps;
    config.system_prompt = document.system_prompt;
    config.compaction_enabled = document.compaction.enabled;
    config.compaction_min_messages = document.compaction.min_messages;
    config.compaction_keep_tail = document.compaction.keep_tail_messages;
    config.cors_origins = document.cors.allowed_origins;
    let security = document
        .security
        .unwrap_or_else(|| crate::settings::SecuritySettings::from_config(&config));
    config.embed_throttle_max = security.embed_token_throttle_max;
    config.embed_throttle_window_secs = security.embed_token_throttle_window_secs;
    config.stream_throttle_max = security.stream_throttle_max;
    config.stream_throttle_window_secs = security.stream_throttle_window_secs;
    config.occupancy_global = security.occupancy_global;
    config.occupancy_per_visitor = security.occupancy_per_visitor;
    config.client_ip_header = security.client_ip_header;
    config.ip_throttle_max = security.ip_throttle_max;
    config.ip_throttle_window_secs = security.ip_throttle_window_secs;
    config
}
