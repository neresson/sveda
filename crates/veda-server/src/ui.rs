use std::path::PathBuf;

use axum::extract::{Form, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::admin::expected_admin_key;
use crate::token;
use crate::{keys_match, AppState};

const COOKIE_NAME: &str = "veda_admin";
const ADMIN_PAGES: &[&str] = &["runtime", "models", "prompts"];

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

pub async fn health() -> Response {
    axum::Json(json!({ "ok": true, "runtime": "rust" })).into_response()
}

pub async fn show(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
) -> Response {
    render(&state, &headers, "runtime", query.error)
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
        return Redirect::to("/veda/admin").into_response();
    };
    if keys_match(&expected, form.key.trim()) {
        return with_session_cookie(&state, Redirect::to("/veda/admin"));
    }
    Redirect::to("/veda/admin?error=1").into_response()
}

pub async fn setup(State(state): State<AppState>, Form(form): Form<SetupForm>) -> Response {
    if expected_admin_key(&state).is_some() {
        return Redirect::to("/veda/admin").into_response();
    }
    let key = form.key.trim();
    if key.chars().count() < 16 {
        return Redirect::to("/veda/admin?error=min").into_response();
    }
    if key != form.key_confirmation.trim() {
        return Redirect::to("/veda/admin?error=confirmed").into_response();
    }
    *state.admin_key.lock().expect("admin key") = Some(key.to_string());
    with_session_cookie(&state, Redirect::to("/veda/admin"))
}

pub async fn logout() -> Response {
    let mut response = Redirect::to("/veda/admin").into_response();
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
            "action": "/veda/admin/setup",
            "error": setup_error(&error),
        })
    } else if !session_ok(state, headers) {
        json!({
            "page": "login",
            "csrf": "",
            "action": "/veda/admin/login",
            "error": if error.is_empty() { "" } else { "invalid" },
        })
    } else {
        settings_payload(state, page)
    };
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
        "saveUrl": "/veda/admin/settings",
        "logoutUrl": "/veda/admin/logout",
        "urls": {
            "dashboard": "/veda/admin",
            "runtime": "/veda/admin/runtime",
            "models": "/veda/admin/models",
            "prompts": "/veda/admin/prompts",
        },
        "settings": state.settings.document().masked(),
    })
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
    <title>Veda admin</title>
    {assets}
</head>
<body class="min-h-screen bg-canvas font-sans text-ink antialiased">
    <div id="veda-admin"></div>
    <script>
        window.VedaAdmin = {json};
    </script>
</body>
</html>"#
    )
}

fn vite_tags() -> String {
    let dist = admin_dist();
    let Ok(raw) = std::fs::read_to_string(dist.join("manifest.json")) else {
        return String::new();
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&raw) else {
        return String::new();
    };
    let Some(entry) = manifest.get("resources/js/admin/app.js") else {
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

pub fn admin_dist() -> PathBuf {
    std::env::var("VEDA_ADMIN_DIST")
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
    token::sign(hmac_key, &format!("veda-admin-session:{admin_key}"))
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
