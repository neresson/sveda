use std::path::PathBuf;

use axum::extract::{Form, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::admin::expected_admin_key;
use crate::token;
use crate::{keys_match, AppState};

const COOKIE_NAME: &str = "sveda_admin";
const ADMIN_PAGES: &[&str] = &[
    "dashboard",
    "usage",
    "runtime",
    "models",
    "mcp",
    "prompts",
    "appearance",
    "sources",
];

#[derive(Debug, Deserialize)]
pub struct AdminQuery {
    #[serde(default)]
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    #[serde(default)]
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct SetupForm {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub key_confirmation: String,
}

#[derive(Debug, Deserialize)]
pub struct EmbedQuery {
    #[serde(default)]
    pub token: String,
}

pub async fn health() -> Response {
    axum::Json(json!({ "ok": true, "runtime": "rust" })).into_response()
}

pub async fn ready(State(state): State<AppState>) -> Response {
    match state.readiness().await {
        Ok(()) => axum::Json(json!({ "ok": true, "runtime": "rust" })).into_response(),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(json!({ "ok": false, "runtime": "rust", "error": error })),
        )
            .into_response(),
    }
}

pub async fn embed_page(
    State(state): State<AppState>,
    Query(query): Query<EmbedQuery>,
) -> Response {
    if !state.config.embed_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }

    let token = query.token.trim();
    if token.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    if token::validate(&state.config.hmac_key, token).is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let public = state.settings.document().public();
    let payload = json!({
        "token": token,
        "origin": "",
        "prefix": "sveda",
        "protocol": "sveda",
        "hideLauncher": true,
        "appearance": public.get("appearance").cloned().unwrap_or(Value::Null),
        "models": public.get("models").cloned().unwrap_or_else(|| json!([])),
    });

    Html(embed_shell(&payload)).into_response()
}

pub async fn show(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
) -> Response {
    render(&state, &headers, "dashboard", query.error)
}

pub async fn section(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(page): Path<String>,
    Query(query): Query<AdminQuery>,
) -> Response {
    if !ADMIN_PAGES.contains(&page.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    render(&state, &headers, &page, query.error)
}

pub async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let Some(expected) = expected_admin_key(&state) else {
        return Redirect::to("/sveda/admin").into_response();
    };
    if keys_match(&expected, form.key.trim()) {
        return with_session_cookie(&state, Redirect::to("/sveda/admin"));
    }
    Redirect::to("/sveda/admin?error=1").into_response()
}

pub async fn setup(State(state): State<AppState>, Form(form): Form<SetupForm>) -> Response {
    if state.config.database_url.is_some() {
        return Redirect::to("/sveda/admin").into_response();
    }
    if expected_admin_key(&state).is_some() {
        return Redirect::to("/sveda/admin").into_response();
    }
    let key = form.key.trim();
    if key.chars().count() < 16 {
        return Redirect::to("/sveda/admin?error=min").into_response();
    }
    if key != form.key_confirmation.trim() {
        return Redirect::to("/sveda/admin?error=confirmed").into_response();
    }
    *state.admin_key.lock().expect("admin key") = Some(key.to_string());
    with_session_cookie(&state, Redirect::to("/sveda/admin"))
}

pub async fn logout() -> Response {
    let mut response = Redirect::to("/sveda/admin").into_response();
    if let Ok(value) = HeaderValue::from_str(&format!(
        "{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"
    )) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

pub fn session_ok(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(expected) = expected_admin_key(state) else {
        return false;
    };
    let Some(cookie) = cookie_value(headers, COOKIE_NAME) else {
        return false;
    };
    keys_match(&session_token(&state.config.hmac_key, &expected), &cookie)
}

fn render(state: &AppState, headers: &HeaderMap, page: &str, error: String) -> Response {
    let configured = expected_admin_key(state).is_some();
    let payload = if !configured {
        json!({
            "page": "setup",
            "csrf": "",
            "action": "/sveda/admin/setup",
            "error": setup_error(&error),
        })
    } else if !session_ok(state, headers) {
        json!({
            "page": "login",
            "csrf": "",
            "action": "/sveda/admin/login",
            "error": if error.is_empty() { "" } else { "invalid" },
        })
    } else {
        settings_payload(state, page)
    };
    if wants_json(headers) {
        if payload.get("page").and_then(Value::as_str) == Some("login")
            || payload.get("page").and_then(Value::as_str) == Some("setup")
        {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        return Json(payload).into_response();
    }
    Html(shell(&payload)).into_response()
}

fn setup_error(error: &str) -> &str {
    match error {
        "min" | "confirmed" | "required" => error,
        _ => "",
    }
}

fn settings_payload(state: &AppState, page: &str) -> Value {
    json!({
        "page": page,
        "csrf": "",
        "saveUrl": "/sveda/admin/settings",
        "logoutUrl": "/sveda/admin/logout",
        "urls": {
            "dashboard": "/sveda/admin",
            "usage": "/sveda/admin/usage",
            "runtime": "/sveda/admin/runtime",
            "models": "/sveda/admin/models",
            "mcp": "/sveda/admin/mcp",
            "prompts": "/sveda/admin/prompts",
            "appearance": "/sveda/admin/appearance",
            "sources": "/sveda/admin/sources",
        },
        "codeIndex": {
            "sources": "/sveda/admin/code-index/sources",
            "progress": "/sveda/admin/code-index/progress",
            "store": "/sveda/admin/code-index/store",
            "sourceBase": "/sveda/admin/code-index/sources",
            "localBrowse": "/sveda/admin/code-index/local-browse",
            "localPreview": "/sveda/admin/code-index/local-preview",
            "estimate": "/sveda/admin/code-index/estimate",
        },
        "appearancePresets": {},
        "settings": state.settings.document().masked(),
        "stats": empty_stats(),
        "usage": empty_usage(),
    })
}

fn empty_stats() -> Value {
    json!({
        "period_days": 14,
        "requests": 0,
        "completed": 0,
        "failed": 0,
        "pending": 0,
        "prompt_tokens": 0,
        "completion_tokens": 0,
        "tokens_used": 0,
        "unsplit_tokens": 0,
        "users": 0,
        "conversations": 0,
        "avg_tokens": 0,
        "success_rate": 0,
        "series": [],
    })
}

fn empty_usage() -> Value {
    json!({
        "by_model": [],
        "requests": {
            "data": [],
            "current_page": 1,
            "last_page": 1,
            "per_page": 25,
            "total": 0,
            "prev_page_url": null,
            "next_page_url": null,
        }
    })
}

fn wants_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("application/json"))
}

fn shell(payload: &Value) -> String {
    let json = serde_json::to_string(payload)
        .unwrap_or_else(|_| "{}".into())
        .replace('<', "\\u003c");
    let assets = vite_tags();
    format!(
        r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Sveda admin</title>
    {assets}
</head>
<body class="min-h-screen bg-canvas font-sans text-ink antialiased">
    <div id="sveda-admin"></div>
    <script>
        window.SvedaAdmin = {json};
    </script>
</body>
</html>"#
    )
}

fn embed_shell(payload: &Value) -> String {
    let json = serde_json::to_string(payload)
        .unwrap_or_else(|_| "{}".into())
        .replace('<', "\\u003c");
    format!(
        r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Sveda</title>
    <link rel="stylesheet" href="/build/sveda/embed.css">
    <style>html,body,#sveda-embed{{margin:0;width:100%;height:100%;background:transparent;overflow:hidden}}</style>
</head>
<body>
    <div id="sveda-embed"></div>
    <script>window.SvedaEmbed = {json};</script>
    <script type="module" src="/build/sveda/embed.js"></script>
</body>
</html>"#
    )
}

fn vite_tags() -> String {
    let dist = admin_dist();
    let Some(manifest) = ["manifest.json", ".vite/manifest.json"]
        .into_iter()
        .find_map(|relative| {
            std::fs::read_to_string(dist.join(relative))
                .ok()
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        })
    else {
        return String::new();
    };
    let Some(entry) = manifest_entry(&manifest) else {
        return String::new();
    };
    let mut tags = String::new();
    if let Some(css) = entry.get("css").and_then(Value::as_array) {
        for item in css {
            if let Some(file) = item.as_str() {
                tags.push_str(&format!(r#"<link rel="stylesheet" href="/build/{file}">"#));
            }
        }
    }
    if let Some(file) = entry.get("file").and_then(Value::as_str) {
        tags.push_str(&format!(
            r#"<script type="module" src="/build/{file}"></script>"#
        ));
    }
    tags
}

fn manifest_entry(manifest: &Value) -> Option<&Value> {
    let object = manifest.as_object()?;
    object.get("resources/js/admin/app.js").or_else(|| {
        object
            .iter()
            .find_map(|(key, value)| key.ends_with("resources/js/admin/app.js").then_some(value))
    })
}

pub fn admin_dist() -> PathBuf {
    std::env::var("SVEDA_ADMIN_DIST")
        .ok()
        .map(|value| PathBuf::from(value.trim()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/runtime/public/build")
        })
}

fn with_session_cookie(state: &AppState, redirect: Redirect) -> Response {
    let Some(expected) = expected_admin_key(state) else {
        return redirect.into_response();
    };
    let token = session_token(&state.config.hmac_key, &expected);
    let mut response = redirect.into_response();
    if let Ok(value) = HeaderValue::from_str(&format!(
        "{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax"
    )) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

fn session_token(hmac_key: &[u8], admin_key: &str) -> String {
    token::sign(hmac_key, &format!("sveda-admin-session:{admin_key}"))
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|part| {
        let part = part.trim();
        let (key, value) = part.split_once('=')?;
        if key.trim() == name {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}
