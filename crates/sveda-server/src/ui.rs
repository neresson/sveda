use std::path::PathBuf;

use axum::extract::{Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::token;
use crate::AppState;

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
    "reports",
];

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
        "The runtime is up. Models and embed settings come from sveda.yaml. The settings API is /admin/settings.",
        "/sveda/health",
        "Health",
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
            "/",
            "Home",
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

pub async fn admin_session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(status) = crate::admin::require_admin(&state, &headers) {
        return status.into_response();
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

fn wants_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("application/json"))
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
    <script>try{{sessionStorage.setItem("sveda.chat-minimized","false")}}catch(e){{}}</script>
    <script type="module" src="/build/sveda/embed.js"></script>
</body>
</html>"#
    )
}

fn vite_style_tags() -> String {
    manifest_assets(false)
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
