use std::path::PathBuf;

use axum::extract::{Form, Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::admin::expected_admin_key;
use crate::token;
use crate::{keys_match, AppState};

const COOKIE_NAME: &str = "sveda_admin";
pub const ADMIN_BASE: &str = "/admin";

pub(crate) fn admin_path(segment: &str) -> String {
    if segment.is_empty() {
        ADMIN_BASE.to_string()
    } else {
        format!("{ADMIN_BASE}/{segment}")
    }
}
pub(crate) const ADMIN_PAGES: &[&str] = &[
    "dashboard",
    "usage",
    "runtime",
    "security",
    "policies",
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
    #[serde(default)]
    pub page: Option<u32>,
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

pub async fn home() -> Html<String> {
    Html(static_page(
        "Sveda",
        "00 / SITE",
        "Under construction",
        "The runtime is up. Configure models and embed settings in the admin panel.",
        ADMIN_BASE,
        "Open admin",
    ))
}

pub async fn not_found(request: Request) -> Response {
    let path = request.uri().path();
    if path.starts_with("/sveda/") || wants_json(request.headers()) {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(json!({ "ok": false, "error": "not_found" })),
        )
            .into_response();
    }
    (
        StatusCode::NOT_FOUND,
        Html(static_page(
            "Not found — Sveda",
            "404",
            "Page not found",
            "This path is not part of the Sveda runtime surface.",
            ADMIN_BASE,
            "Admin panel",
        )),
    )
        .into_response()
}

pub async fn health() -> Response {
    axum::Json(runtime_status(true, None)).into_response()
}

pub async fn ready(State(state): State<AppState>) -> Response {
    match state.readiness().await {
        Ok(()) => axum::Json(runtime_status(true, None)).into_response(),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(runtime_status(false, Some(error))),
        )
            .into_response(),
    }
}

fn runtime_status(ok: bool, error: Option<String>) -> Value {
    let mut body = json!({
        "ok": ok,
        "runtime": "rust",
        "version": runtime_version(),
    });
    if let Some(revision) = runtime_revision() {
        body["revision"] = json!(revision);
    }
    if let Some(error) = error {
        body["error"] = json!(error);
    }
    body
}

fn runtime_version() -> String {
    std::env::var("SVEDA_VERSION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")))
}

fn runtime_revision() -> Option<String> {
    std::env::var("SVEDA_REVISION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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
    render(&state, &headers, "dashboard", query).await
}

pub async fn section(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(page): Path<String>,
    Query(query): Query<AdminQuery>,
) -> Response {
    if !ADMIN_PAGES.contains(&page.as_str()) {
        return (StatusCode::NOT_FOUND, admin_not_found()).into_response();
    }
    render(&state, &headers, &page, query).await
}

pub async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let Some(expected) = expected_admin_key(&state) else {
        return Redirect::to(ADMIN_BASE).into_response();
    };
    if keys_match(&expected, form.key.trim()) {
        return with_session_cookie(&state, Redirect::to(ADMIN_BASE));
    }
    Redirect::to("/admin?error=1").into_response()
}

pub async fn setup(State(state): State<AppState>, Form(form): Form<SetupForm>) -> Response {
    if state.config.database_url.is_some() {
        return Redirect::to(ADMIN_BASE).into_response();
    }
    if expected_admin_key(&state).is_some() {
        return Redirect::to(ADMIN_BASE).into_response();
    }
    let key = form.key.trim();
    if key.chars().count() < 16 {
        return Redirect::to("/admin?error=min").into_response();
    }
    if key != form.key_confirmation.trim() {
        return Redirect::to("/admin?error=confirmed").into_response();
    }
    *state.admin_key.lock().expect("admin key") = Some(key.to_string());
    with_session_cookie(&state, Redirect::to(ADMIN_BASE))
}

pub async fn logout() -> Response {
    let mut response = Redirect::to(ADMIN_BASE).into_response();
    if let Ok(value) = HeaderValue::from_str(&format!(
        "{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"
    )) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

pub async fn admin_session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !session_ok(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !state.config.embed_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }

    let ttl = state.config.token_ttl_seconds.max(60);
    let token = token::issue_admin(&state.config.hmac_key, ttl);
    Json(json!({
        "origin": request_origin(&headers),
        "token": token,
        "expires_in": ttl,
        "appearance": Value::Null,
    }))
    .into_response()
}

fn request_origin(headers: &HeaderMap) -> String {
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            let host = headers
                .get(header::HOST)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("127.0.0.1:8787");
            let proto = headers
                .get("x-forwarded-proto")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("http");
            format!("{proto}://{host}")
        })
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

async fn render(state: &AppState, headers: &HeaderMap, page: &str, query: AdminQuery) -> Response {
    let configured = expected_admin_key(state).is_some();
    let payload = if !configured {
        json!({
            "page": "setup",
            "csrf": "",
            "action": format!("{}/setup", ADMIN_BASE),
            "error": setup_error(&query.error),
        })
    } else if !session_ok(state, headers) {
        json!({
            "page": "login",
            "csrf": "",
            "action": format!("{}/login", ADMIN_BASE),
            "error": if query.error.is_empty() { "" } else { "invalid" },
        })
    } else {
        settings_payload(state, page, query.page.unwrap_or(1)).await
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

async fn settings_payload(state: &AppState, page: &str, page_number: u32) -> Value {
    let stats = match state
        .usage
        .dashboard(sveda_store::DASHBOARD_PERIOD_DAYS)
        .await
    {
        Ok(stats) => stats_json(&stats),
        Err(_) => empty_stats(),
    };
    let usage = match state
        .usage
        .page(page_number, sveda_store::USAGE_PAGE_SIZE)
        .await
    {
        Ok(list) => usage_json(state, &list),
        Err(_) => empty_usage(),
    };
    let settings = state.settings.document();
    let public = settings.public();
    json!({
        "page": page,
        "csrf": "",
        "saveUrl": admin_path("settings"),
        "logoutUrl": admin_path("logout"),
        "urls": {
            "dashboard": ADMIN_BASE,
            "usage": admin_path("usage"),
            "runtime": admin_path("runtime"),
            "security": admin_path("security"),
            "policies": admin_path("policies"),
            "models": admin_path("models"),
            "mcp": admin_path("mcp"),
            "prompts": admin_path("prompts"),
            "appearance": admin_path("appearance"),
            "sources": admin_path("sources"),
        },
        "codeIndex": {
            "sources": admin_path("code-index/sources"),
            "progress": admin_path("code-index/progress"),
            "store": admin_path("code-index/store"),
            "sourceBase": admin_path("code-index/sources"),
            "localBrowse": admin_path("code-index/local-browse"),
            "localPreview": admin_path("code-index/local-preview"),
            "estimate": admin_path("code-index/estimate"),
        },
        "appearancePresets": {},
        "chat": {
            "sessionUrl": admin_path("session"),
            "prefix": "sveda",
            "protocol": "sveda",
            "models": public.get("models").cloned().unwrap_or_else(|| json!([])),
        },
        "settings": settings.masked(),
        "stats": stats,
        "usage": usage,
    })
}

pub(crate) fn stats_json(stats: &sveda_store::DashboardStats) -> Value {
    json!({
        "period_days": stats.period_days,
        "requests": stats.requests,
        "completed": stats.completed,
        "failed": stats.failed,
        "pending": stats.pending,
        "prompt_tokens": stats.prompt_tokens,
        "completion_tokens": stats.completion_tokens,
        "tokens_used": stats.tokens_used,
        "unsplit_tokens": stats.unsplit_tokens,
        "users": stats.users,
        "conversations": stats.conversations,
        "avg_tokens": stats.avg_tokens,
        "success_rate": stats.success_rate,
        "series": stats.series.iter().map(|day| json!({
            "date": day.date.to_string(),
            "requests": day.requests,
            "prompt_tokens": day.prompt_tokens,
            "completion_tokens": day.completion_tokens,
            "tokens_used": day.tokens_used,
            "unsplit_tokens": day.unsplit_tokens,
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn usage_json(state: &AppState, list: &sveda_store::UsageList) -> Value {
    json!({
        "by_model": list.by_model.iter().map(|row| json!({
            "model": row.model,
            "model_label": model_label(state, &row.model),
            "requests": row.requests,
            "tokens_used": row.tokens_used,
        })).collect::<Vec<_>>(),
        "requests": {
            "data": list.rows.iter().map(|row| json!({
                "id": row.id,
                "model": row.model,
                "model_label": model_label(state, &row.model),
                "created_at": row.created_at.to_rfc3339(),
                "tokens_used": row.tokens_used,
            })).collect::<Vec<_>>(),
            "current_page": list.current_page,
            "last_page": list.last_page,
            "per_page": list.per_page,
            "total": list.total,
            "prev_page_url": if list.current_page > 1 {
                Value::String(format!("{}?page={}", admin_path("usage"), list.current_page - 1))
            } else {
                Value::Null
            },
            "next_page_url": if list.current_page < list.last_page {
                Value::String(format!("{}?page={}", admin_path("usage"), list.current_page + 1))
            } else {
                Value::Null
            },
        }
    })
}

fn model_label(state: &AppState, model_id: &str) -> String {
    state
        .catalog()
        .find(model_id)
        .map(|model| {
            if model.label.trim().is_empty() {
                model.id.clone()
            } else {
                model.label.clone()
            }
        })
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| model_id.to_string())
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

fn admin_not_found() -> Html<String> {
    Html(static_page(
        "Not found — Sveda admin",
        "404",
        "Section not found",
        "This admin section does not exist.",
        ADMIN_BASE,
        "Back to dashboard",
    ))
}

fn static_page(
    document_title: &str,
    kicker: &str,
    heading: &str,
    lede: &str,
    link_href: &str,
    link_label: &str,
) -> String {
    let styles = vite_style_tags();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{document_title}</title>
    {styles}
</head>
<body class="min-h-screen bg-canvas font-sans text-ink antialiased">
    <main id="sveda-site" class="relative flex min-h-screen flex-col border border-ink">
        <div class="sveda-grid pointer-events-none absolute inset-0"></div>
        <div class="pointer-events-none absolute -bottom-16 -left-16 size-[280px] rotate-45 border border-ink max-md:hidden"></div>
        <div class="relative flex flex-1 flex-col justify-between px-16 py-16 max-lg:px-5 max-lg:py-8">
            <div class="flex items-center gap-3">
                <span class="flex size-7 items-center justify-center border border-ink">
                    <span class="size-3 bg-ink"></span>
                </span>
                <p class="font-mono text-xs tracking-[0.16em]">Sveda</p>
            </div>
            <div class="max-w-xl">
                <p class="font-mono text-xs tracking-[0.2em] text-muted">{kicker}</p>
                <h1 class="mt-4 font-serif text-5xl tracking-[-0.03em] max-md:text-4xl">{heading}</h1>
                <p class="mt-4 font-serif text-xl text-muted max-lg:text-base">{lede}</p>
                <p class="mt-10">
                    <a href="{link_href}" class="inline-block border border-ink bg-ink px-5 py-3 font-mono text-[11px] tracking-[0.14em] text-canvas no-underline hover:bg-canvas hover:text-ink">{link_label}</a>
                    <a href="/" class="ml-4 inline-block border border-grid px-5 py-3 font-mono text-[11px] tracking-[0.14em] text-ink no-underline hover:bg-grid">Home</a>
                </p>
            </div>
            <p class="font-mono text-[11px] tracking-[0.14em] text-muted max-lg:hidden">sveda-server</p>
        </div>
    </main>
</body>
</html>"#
    )
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

fn vite_style_tags() -> String {
    manifest_assets(false)
}

fn vite_tags() -> String {
    manifest_assets(true)
}

fn manifest_assets(include_script: bool) -> String {
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
    if include_script {
        if let Some(file) = entry.get("file").and_then(Value::as_str) {
            tags.push_str(&format!(
                r#"<script type="module" src="/build/{file}"></script>"#
            ));
        }
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
