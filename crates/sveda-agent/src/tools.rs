use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use sveda_index::WorkspaceIndex;
use sveda_llm::ToolSpec;
use sveda_mcp::{HostMcpClient, McpTool};
use sveda_protocol::{StreamEvent, ToolProgressTask};
use uuid::Uuid;

pub const SEARCH_TOOL_NAME: &str = "search_agent_tools";
pub const SPAWN_TOOL_NAME: &str = "spawn_tasks";
pub const SEARCH_CODE_TOOL_NAME: &str = "search_code";
pub const READ_CODE_TOOL_NAME: &str = "read_code_file";

#[derive(Clone)]
pub struct MemoryBackend {
    results: HashMap<String, Value>,
    pub calls: Arc<Mutex<Vec<(String, Value)>>>,
}

impl MemoryBackend {
    pub fn new(results: HashMap<String, Value>) -> Self {
        Self {
            results,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn call(&self, name: &str, input: Value) -> Value {
        self.calls
            .lock()
            .expect("calls")
            .push((name.to_string(), input));
        self.results.get(name).cloned().unwrap_or_else(
            || json!({ "success": false, "error": format!("Unknown tool {name}.") }),
        )
    }
}

enum CallTarget {
    None,
    Memory(Arc<MemoryBackend>),
    Mcp(Arc<HostMcpClient>),
}

pub struct ToolRuntime {
    target: CallTarget,
    mcp_tools: Vec<McpTool>,
    activated: Mutex<Vec<String>>,
    defer_enabled: bool,
    min_pool: usize,
    always_loaded: Vec<String>,
    max_parallel: usize,
    max_per_turn: usize,
    index: Option<Arc<WorkspaceIndex>>,
}

impl Default for ToolRuntime {
    fn default() -> Self {
        Self {
            target: CallTarget::None,
            mcp_tools: Vec::new(),
            activated: Mutex::new(Vec::new()),
            defer_enabled: true,
            min_pool: 14,
            always_loaded: vec![SPAWN_TOOL_NAME.to_string()],
            max_parallel: 4,
            max_per_turn: 6,
            index: None,
        }
    }
}

impl ToolRuntime {
    pub fn memory(backend: MemoryBackend, tools: Vec<McpTool>) -> Self {
        Self {
            mcp_tools: tools,
            target: CallTarget::Memory(Arc::new(backend)),
            ..Self::default()
        }
    }

    pub fn mcp(client: Arc<HostMcpClient>, tools: Vec<McpTool>) -> Self {
        Self {
            mcp_tools: tools,
            target: CallTarget::Mcp(client),
            ..Self::default()
        }
    }

    pub fn with_defer(mut self, enabled: bool, min_pool: usize) -> Self {
        self.defer_enabled = enabled;
        self.min_pool = min_pool.max(1);
        self
    }

    pub fn with_index(mut self, index: Arc<WorkspaceIndex>) -> Self {
        self.index = Some(index);
        self
    }

    pub fn advertised(&self) -> Vec<ToolSpec> {
        let mut specs = vec![spawn_spec()];
        if self.index.is_some() {
            specs.push(search_code_spec());
            specs.push(read_code_spec());
        }
        let expanded = self.mcp_tools.len() + 1;
        let defer = self.defer_enabled && expanded >= self.min_pool;
        if defer {
            specs.push(search_spec());
            let activated = self.activated.lock().expect("activated").clone();
            for tool in &self.mcp_tools {
                if self.always_loaded.iter().any(|name| name == &tool.name)
                    || activated.iter().any(|name| name == &tool.name)
                {
                    specs.push(mcp_spec(tool));
                }
            }
        } else {
            for tool in &self.mcp_tools {
                specs.push(mcp_spec(tool));
            }
        }
        specs
    }

    pub fn known(&self, name: &str) -> bool {
        name == SPAWN_TOOL_NAME
            || name == SEARCH_TOOL_NAME
            || (self.index.is_some()
                && (name == SEARCH_CODE_TOOL_NAME || name == READ_CODE_TOOL_NAME))
            || self.mcp_tools.iter().any(|tool| tool.name == name)
    }

    pub async fn execute(&self, name: &str, input: Value) -> ToolOutcome {
        if name == SPAWN_TOOL_NAME {
            return self.spawn_tasks(input).await;
        }
        if name == SEARCH_TOOL_NAME {
            return self.search_tools(input);
        }
        if name == SEARCH_CODE_TOOL_NAME {
            return self.search_code(input);
        }
        if name == READ_CODE_TOOL_NAME {
            return self.read_code(input);
        }
        let output = self.call_backend(name, input).await;
        ToolOutcome {
            output,
            events: Vec::new(),
        }
    }

    async fn spawn_tasks(&self, input: Value) -> ToolOutcome {
        let Some(tasks) = input.get("tasks").and_then(Value::as_array) else {
            return ToolOutcome::error("At least one task is required.");
        };
        if tasks.is_empty() {
            return ToolOutcome::error("At least one task is required.");
        }
        if tasks.len() > self.max_per_turn {
            return ToolOutcome::error(&format!(
                "Too many tasks: maximum {} per turn.",
                self.max_per_turn
            ));
        }
        if matches!(self.target, CallTarget::None) {
            return ToolOutcome::error(
                "No subagent resolver registered. Register host MCP tools to run spawn_tasks.",
            );
        }

        let mut parsed = Vec::new();
        for (index, task) in tasks.iter().enumerate() {
            let Some(kind) = task.get("type").and_then(Value::as_str) else {
                return ToolOutcome::error("Each task must have a string \"type\".");
            };
            let input = task.get("input").cloned().unwrap_or_else(|| json!({}));
            let label = task
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(kind)
                .to_string();
            let id = format!("task-{}-{}", index + 1, &Uuid::new_v4().to_string()[..6]);
            parsed.push((id, kind.to_string(), input, label));
        }

        let mut events = Vec::new();
        let mut states: Vec<ToolProgressTask> = parsed
            .iter()
            .map(|(id, _, _, label)| ToolProgressTask {
                id: id.clone(),
                label: label.clone(),
                status: "pending".into(),
                detail: None,
            })
            .collect();
        events.push(progress_event(&states));

        let mut results = Vec::new();
        for chunk in parsed.chunks(self.max_parallel.max(1)) {
            for item in chunk {
                if let Some(state) = states.iter_mut().find(|state| state.id == item.0) {
                    state.status = "running".into();
                }
            }
            events.push(progress_event(&states));

            let futs = chunk.iter().map(|(id, kind, input, label)| {
                let kind = kind.clone();
                let input = input.clone();
                let id = id.clone();
                let label = label.clone();
                async move {
                    let output = self.call_backend(&kind, input).await;
                    (id, label, output)
                }
            });
            let handles = futures_util::future::join_all(futs).await;

            for (id, label, output) in handles {
                let success = output
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                if let Some(state) = states.iter_mut().find(|state| state.id == id) {
                    state.status = if success { "completed" } else { "failed" }.into();
                    if let Some(summary) = output.get("summary").and_then(Value::as_str) {
                        state.detail = Some(summary.chars().take(200).collect());
                    }
                }
                results.push(json!({
                    "id": id,
                    "label": label,
                    "status": states.iter().find(|state| state.id == id).map(|state| state.status.clone()),
                    "success": success,
                    "summary": output.get("summary"),
                    "data": output.get("data"),
                    "error": output.get("error"),
                }));
            }
            events.push(progress_event(&states));
        }

        ToolOutcome {
            output: json!({ "success": true, "data": { "tasks": results } }),
            events,
        }
    }

    fn search_tools(&self, input: Value) -> ToolOutcome {
        let query = input
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if query.is_empty() {
            return ToolOutcome::error("Query is required.");
        }
        let domains: Vec<String> = input
            .get("domains")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let limit = input
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(8)
            .clamp(1, 24) as usize;

        let mut scored: Vec<(f64, &McpTool)> = self
            .mcp_tools
            .iter()
            .filter(|tool| {
                domains.is_empty()
                    || domains
                        .iter()
                        .any(|domain| domain.eq_ignore_ascii_case(&tool.domain))
            })
            .map(|tool| (keyword_score(&query, tool), tool))
            .collect();
        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.1.name.cmp(&right.1.name))
        });
        let filtered: Vec<&McpTool> = scored
            .iter()
            .filter(|(score, _)| *score >= 0.12)
            .map(|(_, tool)| *tool)
            .collect();
        let chosen: Vec<&McpTool> = if filtered.is_empty() {
            scored.iter().take(limit).map(|(_, tool)| *tool).collect()
        } else {
            filtered.into_iter().take(limit).collect()
        };

        let activated: Vec<String> = chosen.iter().map(|tool| tool.name.clone()).collect();
        {
            let mut slot = self.activated.lock().expect("activated");
            for name in &activated {
                if !slot.contains(name) {
                    slot.push(name.clone());
                }
            }
        }

        let tools = chosen
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "domain": tool.domain,
                    "mode": tool.mode,
                    "description": tool.description,
                    "score": keyword_score(&query, tool),
                })
            })
            .collect::<Vec<_>>();

        ToolOutcome {
            output: json!({
                "success": true,
                "data": {
                    "tools": tools,
                    "activated_tools": activated,
                    "backend": "keyword",
                    "hint": if activated.is_empty() {
                        "No matching tools found. Rephrase the query or answer from your own knowledge."
                    } else {
                        "These tools are now available in your next step. Call them directly by name."
                    }
                }
            }),
            events: Vec::new(),
        }
    }

    fn search_code(&self, input: Value) -> ToolOutcome {
        let Some(index) = &self.index else {
            return ToolOutcome::error("Code index is not configured.");
        };
        let query = input
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if query.is_empty() {
            return ToolOutcome::error("Query is required.");
        }
        let limit = input
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(8)
            .clamp(1, 24) as usize;
        let hits: Vec<Value> = index
            .search(&query, limit)
            .into_iter()
            .map(|hit| {
                json!({
                    "path": hit.path,
                    "start_line": hit.start_line,
                    "end_line": hit.end_line,
                    "text": hit.text,
                    "score": hit.score,
                })
            })
            .collect();
        ToolOutcome {
            output: json!({ "success": true, "data": { "hits": hits } }),
            events: Vec::new(),
        }
    }

    fn read_code(&self, input: Value) -> ToolOutcome {
        let Some(index) = &self.index else {
            return ToolOutcome::error("Code index is not configured.");
        };
        let Some(path) = input
            .get("path")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return ToolOutcome::error("Path is required.");
        };
        let offset = input.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let length = input
            .get("length")
            .and_then(Value::as_u64)
            .unwrap_or(4000)
            .clamp(1, 8000) as usize;
        let Some(slice) = index.read_file(path, offset, length) else {
            return ToolOutcome::error("File was not found in the code index.");
        };
        ToolOutcome {
            output: json!({
                "success": true,
                "data": {
                    "path": slice.path,
                    "content": slice.content,
                    "offset": slice.offset,
                    "length": slice.length,
                    "total": slice.total,
                    "has_more": slice.has_more,
                }
            }),
            events: Vec::new(),
        }
    }

    async fn call_backend(&self, name: &str, input: Value) -> Value {
        match &self.target {
            CallTarget::None => {
                json!({ "success": false, "error": format!("Unknown tool {name}.") })
            }
            CallTarget::Memory(backend) => backend.call(name, input).await,
            CallTarget::Mcp(client) => client
                .call_tool(name, input)
                .await
                .unwrap_or_else(|error| json!({ "success": false, "error": error.to_string() })),
        }
    }
}

pub struct ToolOutcome {
    pub output: Value,
    pub events: Vec<StreamEvent>,
}

impl ToolOutcome {
    fn error(message: &str) -> Self {
        Self {
            output: json!({ "success": false, "error": message }),
            events: Vec::new(),
        }
    }
}

fn progress_event(tasks: &[ToolProgressTask]) -> StreamEvent {
    StreamEvent::ToolProgress {
        phase: None,
        tasks: tasks.to_vec(),
        chat_id: None,
        message_id: None,
        timestamp: None,
    }
}

fn spawn_spec() -> ToolSpec {
    ToolSpec {
        name: SPAWN_TOOL_NAME.into(),
        description: "Run multiple independent background tasks in parallel (e.g. searching several sources at once). Each task has a type, an input payload and an optional label. Returns a combined result for all tasks. Use this instead of sequential calls when tasks are independent.".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string" },
                            "input": { "type": "object" },
                            "label": { "type": "string" }
                        },
                        "required": ["type", "input"]
                    }
                }
            },
            "required": ["tasks"]
        }),
    }
}

fn search_code_spec() -> ToolSpec {
    ToolSpec {
        name: SEARCH_CODE_TOOL_NAME.into(),
        description: "Search the local workspace code index with keyword and embedding ranking. Use this before read_code_file.".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer" }
            },
            "required": ["query"]
        }),
    }
}

fn read_code_spec() -> ToolSpec {
    ToolSpec {
        name: READ_CODE_TOOL_NAME.into(),
        description: "Read a file from the local workspace index. First call without offset to get the start of the file. If has_more is true, call again with offset.".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "offset": { "type": "integer" },
                "length": { "type": "integer" }
            },
            "required": ["path"]
        }),
    }
}

fn search_spec() -> ToolSpec {
    ToolSpec {
        name: SEARCH_TOOL_NAME.into(),
        description: "Semantic search over the agent tool catalog. Call this before using business tools that are not already available in the current request. Matching tools become available immediately.".into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "domains": { "type": "array", "items": { "type": "string" } },
                "limit": { "type": "integer" }
            },
            "required": ["query"]
        }),
    }
}

fn mcp_spec(tool: &McpTool) -> ToolSpec {
    ToolSpec {
        name: tool.name.clone(),
        description: tool.description.clone(),
        parameters: tool.input_schema.clone(),
    }
}

fn keyword_score(query: &str, tool: &McpTool) -> f64 {
    let haystack = format!("{} {} {}", tool.name, tool.domain, tool.description).to_lowercase();
    let tokens: Vec<String> = query
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| token.chars().count() >= 2)
        .map(ToOwned::to_owned)
        .collect();
    if tokens.is_empty() {
        return 0.0;
    }
    let hits = tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count();
    hits as f64 / tokens.len() as f64
}

pub fn resource_links(output: &Value) -> Option<Value> {
    let links = output.get("links")?.as_array()?;
    let mut normalized = Vec::new();
    for link in links {
        let Some(label) = link
            .get("label")
            .or_else(|| link.get("name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(url) = link
            .get("url")
            .or_else(|| link.get("link"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let mut item = json!({ "label": label, "url": url });
        if let Some(icon) = link
            .get("icon")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            item["icon"] = json!(icon);
        }
        if let Some(kind) = link
            .get("kind")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            item["kind"] = json!(kind);
        }
        normalized.push(item);
    }
    if normalized.is_empty() {
        None
    } else {
        Some(json!({ "links": normalized }))
    }
}

pub fn normalize_output(output: Value) -> Value {
    if let Value::String(text) = &output {
        serde_json::from_str(text).unwrap_or(output)
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use sveda_index::prepare;

    fn workspace() -> (std::path::PathBuf, WorkspaceIndex) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sveda-agent-index-{nanos}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/limits.rs"),
            "pub fn occupancy_semaphore() {}\n",
        )
        .unwrap();
        let index = prepare(&root);
        (root, index)
    }

    #[tokio::test]
    async fn local_index_tools_are_advertised_and_searchable() {
        let (root, index) = workspace();
        let runtime = ToolRuntime::default().with_index(Arc::new(index));
        let names: Vec<_> = runtime
            .advertised()
            .into_iter()
            .map(|spec| spec.name)
            .collect();
        assert!(names.contains(&SEARCH_CODE_TOOL_NAME.to_string()));
        assert!(names.contains(&READ_CODE_TOOL_NAME.to_string()));
        assert!(runtime.known(SEARCH_CODE_TOOL_NAME));

        let search = runtime
            .execute(
                SEARCH_CODE_TOOL_NAME,
                json!({ "query": "occupancy semaphore" }),
            )
            .await;
        assert_eq!(search.output["success"], true);
        assert_eq!(search.output["data"]["hits"][0]["path"], "src/limits.rs");

        let read = runtime
            .execute(
                READ_CODE_TOOL_NAME,
                json!({ "path": "src/limits.rs", "length": 12 }),
            )
            .await;
        assert_eq!(read.output["success"], true);
        assert_eq!(read.output["data"]["content"], "pub fn occup");
        let _ = fs::remove_dir_all(root);
    }
}
