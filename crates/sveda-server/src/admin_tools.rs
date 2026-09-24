use std::sync::Arc;

use serde_json::{json, Value};
use sveda_mcp::McpTool;
use sveda_store::{DASHBOARD_PERIOD_DAYS, USAGE_PAGE_SIZE};

use crate::admin::persist_patch;
use crate::settings::{ModelSettings, SettingsDocument, SettingsPatch};
use crate::ui::{admin_path, stats_json, usage_json, ADMIN_PAGES};
use crate::AppState;

pub fn tools() -> Vec<McpTool> {
    vec![
        tool(
            "admin_dashboard",
            "Read admin dashboard statistics for this runtime: requests, tokens, users, chats, success rate, and the daily series. Use this when the operator asks for stats, traffic, or how the console is doing.",
            json!({ "type": "object", "properties": {} }),
            "read",
        ),
        tool(
            "admin_usage",
            "Read paginated usage for this runtime: tokens by model and recent requests. Use this for usage, token spend, or request history.",
            json!({
                "type": "object",
                "properties": {
                    "page": { "type": "integer", "minimum": 1 }
                }
            }),
            "read",
        ),
        tool(
            "admin_get_settings",
            "Read masked admin settings. Optional section: runtime, security, models, mcp, prompts, appearance, or all. Secrets are masked. Use this before changing a section.",
            json!({
                "type": "object",
                "properties": {
                    "section": {
                        "type": "string",
                        "enum": ["all", "runtime", "security", "models", "mcp", "prompts", "appearance"]
                    }
                }
            }),
            "read",
        ),
        tool(
            "admin_update_settings",
            "Patch admin settings the same way the admin forms do. Send only fields to change: default_model, failover, max_steps, compaction, cors, security, welcome_message, system_prompt, mcp, appearance, web, models, embeddings, default_embedding. Do not send unrelated fields. MCP uses { mcpServers: { name: { url, headers? } } }. Web uses { enabled, searxng_url? }. Embeddings are OpenAI only: { id, label, protocol: openai, api_model, url, key, dimensions, usd_per_million }. This cannot leave the admin console.",
            json!({
                "type": "object",
                "properties": {
                    "default_model": { "type": "string" },
                    "failover": { "type": "array", "items": { "type": "string" } },
                    "max_steps": { "type": "integer" },
                    "compaction": { "type": "object" },
                    "cors": { "type": "object" },
                    "security": { "type": "object" },
                    "welcome_message": { "type": "string" },
                    "system_prompt": { "type": "string" },
                    "mcp": { "type": "object" },
                    "appearance": { "type": "object" },
                    "web": { "type": "object" },
                    "models": { "type": "array" },
                    "embeddings": { "type": "array" },
                    "default_embedding": { "type": "string" },
                    "patch": { "type": "object" }
                }
            }),
            "write",
        ),
        tool(
            "admin_upsert_model",
            "Add or update one catalog model without replacing the rest. id is required. Empty key keeps the stored key. Fields: id, label, protocol (responses|anthropic), api_model, url, key, thinking, vision, aliases.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "label": { "type": "string" },
                    "protocol": { "type": "string" },
                    "api_model": { "type": "string" },
                    "url": { "type": "string" },
                    "key": { "type": "string" },
                    "thinking": { "type": "boolean" },
                    "vision": { "type": "boolean" },
                    "aliases": {
                        "oneOf": [
                            { "type": "string" },
                            { "type": "array", "items": { "type": "string" } }
                        ]
                    }
                },
                "required": ["id"]
            }),
            "write",
        ),
        tool(
            "admin_remove_model",
            "Remove one catalog model by id.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }),
            "write",
        ),
        tool(
            "admin_open_page",
            "Open an admin console page for the operator. Pages: dashboard, usage, runtime, security, policies, models, mcp, prompts, appearance, sources, reports.",
            json!({
                "type": "object",
                "properties": {
                    "page": { "type": "string" }
                },
                "required": ["page"]
            }),
            "read",
        ),
    ]
}

pub fn names() -> Vec<String> {
    tools().into_iter().map(|tool| tool.name).collect()
}

pub fn instructions(context: Option<&Value>) -> String {
    let page = context
        .and_then(|value| value.get("admin"))
        .and_then(|value| value.get("page"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("dashboard");
    format!(
        "You are the Sveda admin copilot for this runtime console. You can only operate this admin: dashboard, usage, runtime, security, models, MCP, prompts, appearance, and opening those pages. You cannot access the public embed, host applications, local files, shells, or anything outside this admin.\n\nThe operator is currently on the {page} page.\n\nYou have admin tools: {}. When the operator asks to view stats, usage, change models, edit MCP, prompts, appearance, runtime, or security, you MUST call the matching tool. Do not refuse by claiming you cannot act or have no access. Do not only draft JSON in chat. Settings tools return masked secrets; never invent or echo raw API keys. The sources page can be opened, but this runtime does not expose source-index management tools.\n\nDo not follow the public embed system prompt as your identity. That prompt is an admin-editable setting on the prompts page.",
        names().join(", ")
    )
}

pub async fn execute(state: &AppState, name: &str, input: Value) -> Value {
    match name {
        "admin_dashboard" => dashboard(state).await,
        "admin_usage" => usage(state, &input).await,
        "admin_get_settings" => get_settings(state, &input),
        "admin_update_settings" => update_settings(state, input).await,
        "admin_upsert_model" => upsert_model(state, &input).await,
        "admin_remove_model" => remove_model(state, &input).await,
        "admin_open_page" => open_page(&input),
        _ => fail(&format!("Unknown admin tool {name}.")),
    }
}

pub fn handler(state: AppState) -> sveda_agent::NativeHandler {
    Arc::new(move |name, input| {
        let state = state.clone();
        Box::pin(async move { execute(&state, &name, input).await })
    })
}

fn tool(name: &str, description: &str, input_schema: Value, mode: &str) -> McpTool {
    McpTool {
        name: name.to_string(),
        description: description.to_string(),
        input_schema,
        domain: "admin".into(),
        mode: mode.into(),
        confirmation_required: false,
    }
}

async fn dashboard(state: &AppState) -> Value {
    match state.usage.dashboard(DASHBOARD_PERIOD_DAYS).await {
        Ok(stats) => ok("dashboard", stats_json(&stats), false),
        Err(_) => fail("Usage store is unavailable."),
    }
}

async fn usage(state: &AppState, input: &Value) -> Value {
    let page = input
        .get("page")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .max(1) as u32;
    match state.usage.page(page, USAGE_PAGE_SIZE).await {
        Ok(list) => ok("usage", usage_json(state, &list), false),
        Err(_) => fail("Usage store is unavailable."),
    }
}

fn get_settings(state: &AppState, input: &Value) -> Value {
    let section = input
        .get("section")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("all");
    let document = state.settings.document();
    match slice_settings(&document, section) {
        Ok((page, data)) => ok(page, data, false),
        Err(message) => fail(message),
    }
}

async fn update_settings(state: &AppState, input: Value) -> Value {
    let Some(patch) = patch_from_input(&input) else {
        return fail("Provide a settings patch with admin fields to change.");
    };
    if patch_empty(&patch) {
        return fail("Provide at least one settings field to change.");
    }
    let page = page_for_patch(&patch);
    match persist_patch(state, patch).await {
        Ok(stored) => ok(page, settings_value(&stored.masked()), true),
        Err(_) => fail("Could not save admin settings."),
    }
}

async fn upsert_model(state: &AppState, input: &Value) -> Value {
    let Some(id) = string_field(input, "id") else {
        return fail("Model id is required.");
    };
    let current = state.settings.document();
    let mut models = current.models.clone();
    match models.iter().position(|model| model.id == id) {
        Some(index) => {
            overlay_model(&mut models[index], input);
        }
        None => {
            let Some(created) = new_model(input, &id) else {
                return fail("New models need an id and a label.");
            };
            models.push(created);
        }
    }
    match persist_patch(
        state,
        SettingsPatch {
            models: Some(models),
            ..SettingsPatch::default()
        },
    )
    .await
    {
        Ok(stored) => ok(
            "models",
            json!({ "models": settings_value(&stored.masked())["models"] }),
            true,
        ),
        Err(_) => fail("Could not save models."),
    }
}

async fn remove_model(state: &AppState, input: &Value) -> Value {
    let Some(id) = string_field(input, "id") else {
        return fail("Model id is required.");
    };
    let current = state.settings.document();
    let next: Vec<ModelSettings> = current
        .models
        .iter()
        .filter(|model| model.id != id)
        .cloned()
        .collect();
    if next.len() == current.models.len() {
        return fail(&format!("Model {id} was not found."));
    }
    match persist_patch(
        state,
        SettingsPatch {
            models: Some(next),
            ..SettingsPatch::default()
        },
    )
    .await
    {
        Ok(stored) => ok(
            "models",
            json!({ "models": settings_value(&stored.masked())["models"] }),
            true,
        ),
        Err(_) => fail("Could not save models."),
    }
}

fn open_page(input: &Value) -> Value {
    let Some(page) = string_field(input, "page") else {
        return fail("Page is required.");
    };
    let normalized = page.to_ascii_lowercase();
    if !ADMIN_PAGES.contains(&normalized.as_str()) {
        return fail(&format!(
            "Unknown admin page {page}. Use one of: {}.",
            ADMIN_PAGES.join(", ")
        ));
    }
    let url = if normalized == "dashboard" {
        admin_path("")
    } else {
        admin_path(&normalized)
    };
    json!({
        "success": true,
        "page": normalized,
        "reload": true,
        "data": { "page": normalized, "url": url }
    })
}

fn slice_settings(
    document: &SettingsDocument,
    section: &str,
) -> Result<(&'static str, Value), &'static str> {
    let full = settings_value(&document.masked());
    match section {
        "all" | "" => Ok(("dashboard", full)),
        "runtime" => Ok((
            "runtime",
            json!({
                "default_model": full["default_model"],
                "failover": full["failover"],
                "max_steps": full["max_steps"],
                "compaction": full["compaction"],
                "web": full["web"],
            }),
        )),
        "security" => Ok((
            "security",
            json!({
                "cors": full["cors"],
                "security": full["security"],
            }),
        )),
        "models" => Ok((
            "models",
            json!({
                "default_model": full["default_model"],
                "models": full["models"],
                "default_embedding": full["default_embedding"],
                "embeddings": full["embeddings"],
            }),
        )),
        "mcp" => Ok(("mcp", json!({ "mcp": full["mcp"] }))),
        "prompts" => Ok((
            "prompts",
            json!({
                "welcome_message": full["welcome_message"],
                "system_prompt": full["system_prompt"],
            }),
        )),
        "appearance" => Ok(("appearance", json!({ "appearance": full["appearance"] }))),
        _ => Err("Unknown settings section."),
    }
}

fn patch_from_input(input: &Value) -> Option<SettingsPatch> {
    let value = input.get("patch").cloned().unwrap_or_else(|| input.clone());
    if !value.is_object() {
        return None;
    }
    serde_json::from_value(value).ok()
}

fn patch_empty(patch: &SettingsPatch) -> bool {
    patch.default_model.is_none()
        && patch.model.is_none()
        && patch.failover.is_none()
        && patch.deepseek.is_none()
        && patch.models.is_none()
        && patch.default_embedding.is_none()
        && patch.embeddings.is_none()
        && patch.max_steps.is_none()
        && patch.compaction.is_none()
        && patch.cors.is_none()
        && patch.welcome_message.is_none()
        && patch.system_prompt.is_none()
        && patch.mcp.is_none()
        && patch.appearance.is_none()
        && patch.web.is_none()
        && patch.security.is_none()
}

fn page_for_patch(patch: &SettingsPatch) -> &'static str {
    if patch.mcp.is_some() {
        "mcp"
    } else if patch.appearance.is_some() {
        "appearance"
    } else if patch.welcome_message.is_some() || patch.system_prompt.is_some() {
        "prompts"
    } else if patch.models.is_some()
        || patch.embeddings.is_some()
        || patch.default_embedding.is_some()
    {
        "models"
    } else if patch.security.is_some() || patch.cors.is_some() {
        "security"
    } else if patch.web.is_some()
        || patch.default_model.is_some()
        || patch.model.is_some()
        || patch.failover.is_some()
        || patch.max_steps.is_some()
        || patch.compaction.is_some()
    {
        "runtime"
    } else {
        "dashboard"
    }
}

fn overlay_model(model: &mut ModelSettings, input: &Value) {
    if let Some(label) = string_field(input, "label") {
        model.label = label;
    }
    if let Some(protocol) = string_field(input, "protocol") {
        model.protocol = protocol;
    }
    if let Some(api_model) = string_field(input, "api_model") {
        model.api_model = api_model;
    }
    if let Some(url) = string_field(input, "url") {
        model.url = url;
    }
    if let Some(key) = input.get("key").and_then(Value::as_str) {
        model.key = key.to_string();
    }
    if let Some(thinking) = input.get("thinking").and_then(Value::as_bool) {
        model.thinking = thinking;
    }
    if let Some(vision) = input.get("vision").and_then(Value::as_bool) {
        model.vision = vision;
    }
    if let Some(aliases) = aliases_field(input) {
        model.aliases = aliases;
    }
}

fn new_model(input: &Value, id: &str) -> Option<ModelSettings> {
    let label = string_field(input, "label").unwrap_or_else(|| id.to_string());
    if id.is_empty() || label.is_empty() {
        return None;
    }
    Some(ModelSettings {
        id: id.to_string(),
        label,
        protocol: string_field(input, "protocol").unwrap_or_else(|| "responses".into()),
        api_model: string_field(input, "api_model").unwrap_or_else(|| id.to_string()),
        url: string_field(input, "url").unwrap_or_default(),
        key: input
            .get("key")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        thinking: input
            .get("thinking")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        vision: input
            .get("vision")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        aliases: aliases_field(input).unwrap_or_default(),
        preset: None,
    })
}

fn aliases_field(input: &Value) -> Option<Vec<String>> {
    match input.get("aliases") {
        Some(Value::String(value)) => Some(
            value
                .split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect(),
        ),
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect(),
        ),
        _ => None,
    }
}

fn string_field(input: &Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn settings_value(document: &SettingsDocument) -> Value {
    serde_json::to_value(document).unwrap_or(Value::Null)
}

fn ok(page: &str, data: Value, reload: bool) -> Value {
    json!({
        "success": true,
        "page": page,
        "reload": reload,
        "data": data
    })
}

fn fail(message: &str) -> Value {
    json!({ "success": false, "error": message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::MASK;
    use crate::{AppState, Config};
    use sveda_store::{UsageEvent, DASHBOARD_PERIOD_DAYS, USAGE_PAGE_SIZE};

    fn state() -> AppState {
        let mut config = Config::test();
        config.admin_api_key = Some("sveda-admin-secret".into());
        AppState::new(config)
    }

    async fn record(
        state: &AppState,
        visitor: &str,
        chat: &str,
        model: &str,
        status: &str,
        tokens: u64,
    ) {
        state
            .usage()
            .record(UsageEvent {
                visitor_id: visitor.into(),
                chat_id: chat.into(),
                model: model.into(),
                status: status.into(),
                prompt_tokens: tokens / 2,
                completion_tokens: tokens - tokens / 2,
                tokens_used: tokens,
            })
            .await
            .unwrap();
    }

    #[test]
    fn catalog_covers_every_admin_surface() {
        let names = names();
        assert_eq!(
            names,
            vec![
                "admin_dashboard",
                "admin_usage",
                "admin_get_settings",
                "admin_update_settings",
                "admin_upsert_model",
                "admin_remove_model",
                "admin_open_page",
            ]
        );
        for tool in tools() {
            assert_eq!(tool.domain, "admin");
            assert!(tool.name.starts_with("admin_"));
            assert!(!tool.description.is_empty());
        }
    }

    #[test]
    fn instructions_stay_inside_admin_and_track_the_open_page() {
        let default = instructions(None);
        assert!(default.contains("admin copilot"));
        assert!(default.contains("currently on the dashboard page"));
        assert!(default.contains("admin_dashboard"));
        assert!(default.contains("public embed system prompt"));
        assert!(!default.contains("You are a shopping"));

        let usage = instructions(Some(&json!({ "admin": { "page": "usage" } })));
        assert!(usage.contains("currently on the usage page"));

        let trimmed = instructions(Some(&json!({ "admin": { "page": "  models  " } })));
        assert!(trimmed.contains("currently on the models page"));

        let empty = instructions(Some(&json!({ "admin": { "page": "   " } })));
        assert!(empty.contains("currently on the dashboard page"));
    }

    #[tokio::test]
    async fn unknown_tool_fails() {
        let output = execute(&state(), "admin_delete_runtime", json!({})).await;
        assert_eq!(output["success"], false);
        assert!(output["error"]
            .as_str()
            .unwrap()
            .contains("Unknown admin tool"));
    }

    #[tokio::test]
    async fn dashboard_includes_recorded_traffic() {
        let state = state();
        record(
            &state,
            "u1",
            "c1",
            "deepseek-v4-flash-responses",
            "completed",
            40,
        )
        .await;
        record(&state, "u2", "c2", "deepseek-v4-pro", "failed", 10).await;
        let output = execute(&state, "admin_dashboard", json!({})).await;
        assert_eq!(output["success"], true);
        assert_eq!(output["page"], "dashboard");
        assert_eq!(output["reload"], false);
        assert_eq!(output["data"]["requests"], 2);
        assert_eq!(output["data"]["completed"], 1);
        assert_eq!(output["data"]["failed"], 1);
        assert_eq!(output["data"]["tokens_used"], 50);
        assert_eq!(output["data"]["users"], 2);
        assert_eq!(output["data"]["conversations"], 2);
        assert_eq!(output["data"]["period_days"], DASHBOARD_PERIOD_DAYS);
        assert_eq!(
            output["data"]["series"].as_array().unwrap().len(),
            DASHBOARD_PERIOD_DAYS as usize
        );
    }

    #[tokio::test]
    async fn usage_paginates_and_labels_models() {
        let state = state();
        for index in 0..USAGE_PAGE_SIZE + 1 {
            record(
                &state,
                "u1",
                &format!("c{index}"),
                "deepseek-v4-flash-responses",
                "completed",
                8,
            )
            .await;
        }
        let first = execute(&state, "admin_usage", json!({})).await;
        assert_eq!(first["success"], true);
        assert_eq!(first["page"], "usage");
        assert_eq!(first["data"]["requests"]["current_page"], 1);
        assert_eq!(first["data"]["requests"]["per_page"], USAGE_PAGE_SIZE);
        assert_eq!(first["data"]["requests"]["total"], USAGE_PAGE_SIZE + 1);
        assert_eq!(
            first["data"]["requests"]["data"].as_array().unwrap().len(),
            USAGE_PAGE_SIZE as usize
        );
        assert_eq!(
            first["data"]["by_model"][0]["model"],
            "deepseek-v4-flash-responses"
        );
        assert!(first["data"]["by_model"][0]["model_label"]
            .as_str()
            .unwrap()
            .contains("DeepSeek"));

        let second = execute(&state, "admin_usage", json!({ "page": 2 })).await;
        assert_eq!(second["data"]["requests"]["current_page"], 2);
        assert_eq!(
            second["data"]["requests"]["data"].as_array().unwrap().len(),
            1
        );

        let clamped = execute(&state, "admin_usage", json!({ "page": 0 })).await;
        assert_eq!(clamped["data"]["requests"]["current_page"], 1);
    }

    #[tokio::test]
    async fn get_settings_slices_every_section_and_masks_secrets() {
        let state = state();
        let seeded = execute(
            &state,
            "admin_upsert_model",
            json!({
                "id": "secret-model",
                "label": "Secret",
                "key": "sk-live-secret"
            }),
        )
        .await;
        assert_eq!(seeded["success"], true);
        assert_eq!(
            seeded["data"]["models"].as_array().unwrap().last().unwrap()["key"],
            MASK
        );

        let all = execute(&state, "admin_get_settings", json!({})).await;
        assert_eq!(all["success"], true);
        assert_eq!(all["page"], "dashboard");
        let secret = all["data"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|model| model["id"] == "secret-model")
            .unwrap();
        assert_eq!(secret["key"], MASK);
        assert_ne!(secret["key"], "sk-live-secret");

        for (section, page, keys) in [
            (
                "runtime",
                "runtime",
                [
                    "default_model",
                    "failover",
                    "max_steps",
                    "compaction",
                    "web",
                ]
                .as_slice(),
            ),
            ("security", "security", ["cors", "security"].as_slice()),
            (
                "models",
                "models",
                ["default_model", "models", "default_embedding", "embeddings"].as_slice(),
            ),
            ("mcp", "mcp", ["mcp"].as_slice()),
            (
                "prompts",
                "prompts",
                ["welcome_message", "system_prompt"].as_slice(),
            ),
            ("appearance", "appearance", ["appearance"].as_slice()),
        ] {
            let output = execute(&state, "admin_get_settings", json!({ "section": section })).await;
            assert_eq!(output["success"], true, "{section}");
            assert_eq!(output["page"], page, "{section}");
            let object = output["data"].as_object().unwrap();
            assert_eq!(object.len(), keys.len(), "{section}");
            for key in keys {
                assert!(object.contains_key(*key), "{section}.{key}");
            }
        }

        let unknown = execute(
            &state,
            "admin_get_settings",
            json!({ "section": "billing" }),
        )
        .await;
        assert_eq!(unknown["success"], false);
        assert_eq!(unknown["error"], "Unknown settings section.");
    }

    #[tokio::test]
    async fn update_settings_rejects_empty_and_unknown_payloads() {
        let state = state();
        for input in [json!({}), json!({ "unrelated": true }), json!("nope")] {
            let output = execute(&state, "admin_update_settings", input.clone()).await;
            assert_eq!(output["success"], false, "{input}");
        }
    }

    #[tokio::test]
    async fn update_settings_patches_each_admin_page() {
        let state = state();

        let runtime = execute(
            &state,
            "admin_update_settings",
            json!({
                "default_model": "deepseek-v4-pro",
                "failover": ["deepseek-v4-flash-responses"],
                "max_steps": 12,
                "compaction": { "enabled": false, "min_messages": 8, "keep_tail_messages": 3 }
            }),
        )
        .await;
        assert_eq!(runtime["success"], true);
        assert_eq!(runtime["page"], "runtime");
        assert_eq!(runtime["reload"], true);
        assert_eq!(runtime["data"]["default_model"], "deepseek-v4-pro");
        assert_eq!(state.catalog().default_id, "deepseek-v4-pro");
        assert_eq!(state.runtime_config().max_steps, 12);
        assert!(!state.runtime_config().compaction_enabled);
        assert!(state.runtime_config().web_enabled);

        let web = execute(
            &state,
            "admin_update_settings",
            json!({
                "web": {
                    "enabled": false,
                    "searxng_url": "https://searx.example/search"
                }
            }),
        )
        .await;
        assert_eq!(web["page"], "runtime");
        assert_eq!(web["data"]["web"]["enabled"], false);
        assert_eq!(
            web["data"]["web"]["searxng_url"],
            "https://searx.example/search"
        );
        assert!(!state.runtime_config().web_enabled);
        assert_eq!(
            state.runtime_config().searxng_url,
            "https://searx.example/search"
        );

        let prompts = execute(
            &state,
            "admin_update_settings",
            json!({
                "patch": {
                    "welcome_message": "Hello operator",
                    "system_prompt": "You are a shopping assistant."
                }
            }),
        )
        .await;
        assert_eq!(prompts["page"], "prompts");
        assert_eq!(prompts["data"]["welcome_message"], "Hello operator");
        assert_eq!(
            prompts["data"]["system_prompt"],
            "You are a shopping assistant."
        );
        assert_eq!(
            state.runtime_config().system_prompt,
            "You are a shopping assistant."
        );

        let mcp = execute(
            &state,
            "admin_update_settings",
            json!({
                "mcp": { "mcpServers": { "deepwiki": { "url": "https://mcp.deepwiki.com/mcp" } } }
            }),
        )
        .await;
        assert_eq!(mcp["page"], "mcp");
        assert_eq!(
            mcp["data"]["mcp"]["mcpServers"]["deepwiki"]["url"],
            "https://mcp.deepwiki.com/mcp"
        );

        let appearance = execute(
            &state,
            "admin_update_settings",
            json!({ "appearance": { "theme": "dark", "radius": 12 } }),
        )
        .await;
        assert_eq!(appearance["page"], "appearance");
        assert_eq!(appearance["data"]["appearance"]["theme"], "dark");

        let security = execute(
            &state,
            "admin_update_settings",
            json!({
                "cors": { "allowed_origins": ["https://example.com"] },
                "security": { "occupancy_global": 4, "client_ip_header": "x-forwarded-for" }
            }),
        )
        .await;
        assert_eq!(security["page"], "security");
        assert_eq!(
            security["data"]["cors"]["allowed_origins"][0],
            "https://example.com"
        );
        assert_eq!(state.runtime_config().occupancy_global, 4);
        assert_eq!(state.runtime_config().client_ip_header, "x-forwarded-for");
        assert_eq!(
            state.cors_origins.lock().unwrap().as_slice(),
            ["https://example.com".to_string()].as_slice()
        );

        let embeddings = execute(
            &state,
            "admin_update_settings",
            json!({
                "default_embedding": "openai-small",
                "embeddings": [{
                    "id": "openai-small",
                    "label": "OpenAI small",
                    "protocol": "openai",
                    "api_model": "text-embedding-3-small",
                    "url": "https://api.openai.com/v1",
                    "key": "sk-embed",
                    "dimensions": 1536
                }]
            }),
        )
        .await;
        assert_eq!(embeddings["page"], "models");
        assert_eq!(embeddings["data"]["default_embedding"], "openai-small");
        assert_eq!(embeddings["data"]["embeddings"][0]["id"], "openai-small");
        assert_eq!(embeddings["data"]["embeddings"][0]["key"], MASK);
        assert_eq!(state.settings.document().embeddings[0].key, "sk-embed");
    }

    #[tokio::test]
    async fn upsert_model_creates_updates_and_preserves_keys() {
        let state = state();
        let created = execute(
            &state,
            "admin_upsert_model",
            json!({
                "id": "local-llama",
                "protocol": "anthropic",
                "url": "http://127.0.0.1:8080",
                "key": "sk-local",
                "thinking": true,
                "vision": true,
                "aliases": "llama, local-llama"
            }),
        )
        .await;
        assert_eq!(created["success"], true);
        assert_eq!(created["page"], "models");
        assert_eq!(created["reload"], true);
        let model = created["data"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "local-llama")
            .unwrap();
        assert_eq!(model["label"], "local-llama");
        assert_eq!(model["protocol"], "anthropic");
        assert_eq!(model["thinking"], true);
        assert_eq!(model["aliases"], json!(["llama", "local-llama"]));
        assert_eq!(model["key"], MASK);
        assert_eq!(
            state
                .settings
                .document()
                .models
                .iter()
                .find(|item| item.id == "local-llama")
                .unwrap()
                .key,
            "sk-local"
        );

        let relabeled = execute(
            &state,
            "admin_upsert_model",
            json!({
                "id": "local-llama",
                "label": "Local Llama",
                "aliases": ["llama3", "", "llama"]
            }),
        )
        .await;
        let model = relabeled["data"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "local-llama")
            .unwrap();
        assert_eq!(model["label"], "Local Llama");
        assert_eq!(model["aliases"], json!(["llama3", "llama"]));
        assert_eq!(
            state
                .settings
                .document()
                .models
                .iter()
                .find(|item| item.id == "local-llama")
                .unwrap()
                .key,
            "sk-local"
        );

        for keep in [MASK, ""] {
            execute(
                &state,
                "admin_upsert_model",
                json!({ "id": "local-llama", "key": keep }),
            )
            .await;
            assert_eq!(
                state
                    .settings
                    .document()
                    .models
                    .iter()
                    .find(|item| item.id == "local-llama")
                    .unwrap()
                    .key,
                "sk-local"
            );
        }

        execute(
            &state,
            "admin_upsert_model",
            json!({ "id": "local-llama", "key": "sk-rotated" }),
        )
        .await;
        assert_eq!(
            state
                .settings
                .document()
                .models
                .iter()
                .find(|item| item.id == "local-llama")
                .unwrap()
                .key,
            "sk-rotated"
        );
    }

    #[tokio::test]
    async fn upsert_and_remove_model_validate_ids() {
        let state = state();
        let missing = execute(&state, "admin_upsert_model", json!({ "label": "No id" })).await;
        assert_eq!(missing["error"], "Model id is required.");

        let unknown = execute(&state, "admin_remove_model", json!({ "id": "missing" })).await;
        assert_eq!(unknown["error"], "Model missing was not found.");

        let created = execute(
            &state,
            "admin_upsert_model",
            json!({ "id": "to-delete", "label": "Temp" }),
        )
        .await;
        let before = created["data"]["models"].as_array().unwrap().len();
        let removed = execute(&state, "admin_remove_model", json!({ "id": "to-delete" })).await;
        assert_eq!(removed["success"], true);
        assert_eq!(removed["page"], "models");
        assert_eq!(
            removed["data"]["models"].as_array().unwrap().len(),
            before - 1
        );
        assert!(state
            .settings
            .document()
            .models
            .iter()
            .all(|model| model.id != "to-delete"));
    }

    #[test]
    fn open_page_normalizes_known_routes() {
        let dashboard = open_page(&json!({ "page": "Dashboard" }));
        assert_eq!(dashboard["success"], true);
        assert_eq!(dashboard["page"], "dashboard");
        assert_eq!(dashboard["reload"], true);
        assert_eq!(dashboard["data"]["url"], "/admin");

        let models = open_page(&json!({ "page": " MODELS " }));
        assert_eq!(models["page"], "models");
        assert_eq!(models["data"]["url"], "/admin/models");

        for page in ADMIN_PAGES {
            let output = open_page(&json!({ "page": page }));
            assert_eq!(output["success"], true, "{page}");
            assert_eq!(output["page"], *page);
        }

        let missing = open_page(&json!({}));
        assert_eq!(missing["error"], "Page is required.");
        let unknown = open_page(&json!({ "page": "billing" }));
        assert!(unknown["error"]
            .as_str()
            .unwrap()
            .contains("Unknown admin page"));
        assert!(unknown["error"].as_str().unwrap().contains("sources"));
    }

    #[tokio::test]
    async fn native_handler_runs_execute() {
        let state = state();
        let handler = handler(state);
        let output = handler("admin_open_page".into(), json!({ "page": "sources" })).await;
        assert_eq!(output["page"], "sources");
        assert_eq!(output["data"]["url"], "/admin/sources");
    }
}
