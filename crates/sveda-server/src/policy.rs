use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sveda_mcp::McpTool;

pub const MAX_POLICY_NAME_LEN: usize = 64;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<ClientCapability>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCapability {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default, rename = "max_mode")]
    pub max_mode: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientCapability {
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveCapabilities {
    pub web: bool,
    pub code: bool,
    pub mcp: McpCapability,
    pub client: ClientCapability,
}

impl EffectiveCapabilities {
    pub fn unrestricted() -> Self {
        Self {
            web: true,
            code: true,
            mcp: McpCapability {
                allow: vec!["*".to_string()],
                domains: Vec::new(),
                max_mode: Some("delete".to_string()),
            },
            client: ClientCapability {
                allow: vec!["*".to_string()],
            },
        }
    }

    pub fn to_public_json(&self, restricted: bool) -> Value {
        if !restricted {
            return json!({ "restricted": false });
        }
        json!({
            "restricted": true,
            "web": self.web,
            "code": self.code,
            "mcp": {
                "allow": self.mcp.allow,
                "domains": self.mcp.domains,
                "max_mode": self.mcp.max_mode,
            },
            "client": {
                "allow": self.client.allow,
            },
        })
    }
}

pub fn policy_name_valid(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed.len() <= MAX_POLICY_NAME_LEN
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

pub fn claims_restricted(policy: Option<&str>, grants: Option<&CapabilityPolicy>) -> bool {
    policy.map(str::trim).is_some_and(|name| !name.is_empty()) || grants.is_some()
}

pub fn resolve_effective(
    policies: &BTreeMap<String, CapabilityPolicy>,
    policy_name: Option<&str>,
    grants: Option<&CapabilityPolicy>,
) -> Option<Result<EffectiveCapabilities, String>> {
    if !claims_restricted(policy_name, grants) {
        return None;
    }

    let named = policy_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            policies
                .get(name)
                .cloned()
                .ok_or_else(|| format!("unknown policy: {name}"))
        });

    let base = match named {
        Some(Err(message)) => return Some(Err(message)),
        Some(Ok(policy)) => policy,
        None => CapabilityPolicy::default(),
    };

    let merged = match grants {
        Some(extra) => intersect_policies(&base, extra),
        None => base,
    };

    Some(Ok(normalize_restricted(&merged)))
}

fn normalize_restricted(policy: &CapabilityPolicy) -> EffectiveCapabilities {
    EffectiveCapabilities {
        web: policy.web.unwrap_or(false),
        code: policy.code.unwrap_or(false),
        mcp: policy.mcp.clone().unwrap_or_default(),
        client: policy.client.clone().unwrap_or_default(),
    }
}

fn intersect_policies(base: &CapabilityPolicy, grants: &CapabilityPolicy) -> CapabilityPolicy {
    CapabilityPolicy {
        web: intersect_bool(base.web, grants.web),
        code: intersect_bool(base.code, grants.code),
        mcp: intersect_mcp(base.mcp.as_ref(), grants.mcp.as_ref()),
        client: intersect_client(base.client.as_ref(), grants.client.as_ref()),
    }
}

fn intersect_bool(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a && b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn intersect_mcp(
    left: Option<&McpCapability>,
    right: Option<&McpCapability>,
) -> Option<McpCapability> {
    match (left, right) {
        (None, None) => None,
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (Some(a), Some(b)) => Some(McpCapability {
            allow: intersect_string_lists(&a.allow, &b.allow),
            domains: intersect_string_lists(&a.domains, &b.domains),
            max_mode: intersect_max_mode(a.max_mode.as_deref(), b.max_mode.as_deref()),
        }),
    }
}

fn intersect_client(
    left: Option<&ClientCapability>,
    right: Option<&ClientCapability>,
) -> Option<ClientCapability> {
    match (left, right) {
        (None, None) => None,
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (Some(a), Some(b)) => Some(ClientCapability {
            allow: intersect_string_lists(&a.allow, &b.allow),
        }),
    }
}

fn intersect_string_lists(left: &[String], right: &[String]) -> Vec<String> {
    if left.is_empty() {
        return right.to_vec();
    }
    if right.is_empty() {
        return left.to_vec();
    }
    left.iter()
        .filter(|item| right.iter().any(|other| patterns_equivalent(item, other)))
        .cloned()
        .collect()
}

fn patterns_equivalent(left: &str, right: &str) -> bool {
    left == right || left == "*" || right == "*"
}

fn intersect_max_mode(left: Option<&str>, right: Option<&str>) -> Option<String> {
    match (left, right) {
        (Some(a), Some(b)) => Some(min_mode(a, b).to_string()),
        (Some(a), None) => Some(a.to_string()),
        (None, Some(b)) => Some(b.to_string()),
        (None, None) => None,
    }
}

pub fn filter_mcp_tools(
    tools: Vec<McpTool>,
    rules: &McpCapability,
    restricted: bool,
) -> Vec<McpTool> {
    tools
        .into_iter()
        .filter(|tool| mcp_tool_allowed(tool, rules, restricted))
        .collect()
}

fn mcp_tool_allowed(tool: &McpTool, rules: &McpCapability, restricted: bool) -> bool {
    if restricted && rules.allow.is_empty() {
        return false;
    }
    if !rules.allow.is_empty() && !rules.allow.iter().any(|pat| glob_match(&tool.name, pat)) {
        return false;
    }
    if !rules.domains.is_empty()
        && !rules
            .domains
            .iter()
            .any(|pat| glob_match(&tool.domain, pat))
    {
        return false;
    }
    if let Some(max_mode) = rules.max_mode.as_deref() {
        if mode_rank(&tool.mode) > mode_rank(max_mode) {
            return false;
        }
    }
    true
}

pub fn filter_client_tools(
    tools: Option<&Vec<Value>>,
    rules: &ClientCapability,
    restricted: bool,
) -> Option<Vec<Value>> {
    let Some(tools) = tools else {
        return None;
    };
    if !restricted {
        return Some(tools.clone());
    }
    if rules.allow.is_empty() {
        return Some(Vec::new());
    }
    Some(
        tools
            .iter()
            .filter(|tool| {
                tool_name_from_client(tool)
                    .is_some_and(|name| rules.allow.iter().any(|pat| glob_match(&name, pat)))
            })
            .cloned()
            .collect(),
    )
}

fn tool_name_from_client(tool: &Value) -> Option<String> {
    tool.get("name")
        .or_else(|| tool.get("toolName"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn glob_match(value: &str, pattern: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        if prefix.is_empty() {
            return true;
        }
        return value.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
    }
    value == pattern
}

fn mode_rank(mode: &str) -> u8 {
    match mode.trim().to_ascii_lowercase().as_str() {
        "delete" => 3,
        "write" => 2,
        _ => 1,
    }
}

fn min_mode(left: &str, right: &str) -> &'static str {
    if mode_rank(left) <= mode_rank(right) {
        match left.trim().to_ascii_lowercase().as_str() {
            "delete" => "delete",
            "write" => "write",
            _ => "read",
        }
    } else {
        match right.trim().to_ascii_lowercase().as_str() {
            "delete" => "delete",
            "write" => "write",
            _ => "read",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mcp_tool(name: &str, domain: &str, mode: &str) -> McpTool {
        McpTool {
            name: name.into(),
            description: String::new(),
            input_schema: json!({ "type": "object" }),
            domain: domain.into(),
            mode: mode.into(),
            confirmation_required: false,
        }
    }

    #[test]
    fn glob_patterns_work() {
        assert!(glob_match("tickets.search", "tickets.*"));
        assert!(glob_match("ui_confirm", "*confirm"));
        assert!(!glob_match("billing.search", "tickets.*"));
    }

    #[test]
    fn glob_match_covers_star_prefix_suffix_exact_and_empty() {
        assert!(glob_match("anything", "*"));
        assert!(glob_match("search_posts", "search*"));
        assert!(glob_match("search_posts", "*posts"));
        assert!(glob_match("exact_tool", "exact_tool"));
        assert!(!glob_match("exact_tool", "other_tool"));
        assert!(!glob_match("search_posts", ""));
        assert!(!glob_match("search_posts", "   "));
        assert!(!glob_match("prefix_only", "*missing"));
        assert!(!glob_match("only_suffix", "missing*"));
    }

    #[test]
    fn max_mode_read_drops_write_and_delete_tools() {
        let rules = McpCapability {
            allow: vec!["*".into()],
            domains: Vec::new(),
            max_mode: Some("read".into()),
        };
        let tools = vec![
            mcp_tool("list", "blog", "read"),
            mcp_tool("update", "blog", "write"),
            mcp_tool("remove", "blog", "delete"),
        ];
        let filtered = filter_mcp_tools(tools, &rules, true);
        let names: Vec<_> = filtered.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names, vec!["list"]);
    }

    #[test]
    fn mcp_filter_applies_domain_and_mode() {
        let rules = McpCapability {
            allow: vec!["search_*".into()],
            domains: vec!["docs".into()],
            max_mode: Some("write".into()),
        };
        let tools = vec![
            mcp_tool("search_posts", "docs", "read"),
            mcp_tool("search_posts", "billing", "read"),
            mcp_tool("search_posts", "docs", "delete"),
            mcp_tool("create_post", "docs", "write"),
        ];
        let filtered = filter_mcp_tools(tools, &rules, true);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "search_posts");
        assert_eq!(filtered[0].domain, "docs");
        assert_eq!(filtered[0].mode, "read");
    }

    #[test]
    fn restricted_empty_mcp_allow_blocks_all() {
        let rules = McpCapability::default();
        let tools = vec![mcp_tool("echo", "host", "read")];
        assert!(filter_mcp_tools(tools, &rules, true).is_empty());
    }

    #[test]
    fn filter_client_tools_strips_disallowed_names() {
        let tools = vec![
            json!({ "name": "allowed_tool", "description": "ok" }),
            json!({ "name": "blocked_tool", "description": "no" }),
            json!({ "toolName": "also_allowed", "description": "ok" }),
        ];
        let rules = ClientCapability {
            allow: vec!["allowed_tool".into(), "also_*".into()],
        };
        let filtered = filter_client_tools(Some(&tools), &rules, true).expect("filtered");
        let names: Vec<_> = filtered.iter().filter_map(tool_name_from_client).collect();
        assert_eq!(names, vec!["allowed_tool", "also_allowed"]);
    }

    #[test]
    fn filter_client_tools_empty_allow_under_restricted_policy_drops_all() {
        let tools = vec![json!({ "name": "any_tool" })];
        let filtered = filter_client_tools(Some(&tools), &ClientCapability::default(), true)
            .expect("filtered");
        assert!(filtered.is_empty());
    }

    #[test]
    fn intersect_tightens_web() {
        let base = CapabilityPolicy {
            web: Some(true),
            code: Some(true),
            mcp: None,
            client: None,
        };
        let grants = CapabilityPolicy {
            web: Some(false),
            code: None,
            mcp: None,
            client: None,
        };
        let merged = intersect_policies(&base, &grants);
        assert_eq!(merged.web, Some(false));
        assert_eq!(merged.code, Some(true));
    }

    #[test]
    fn grants_cannot_widen_web_from_false_to_true() {
        let mut policies = BTreeMap::new();
        policies.insert(
            "reader".into(),
            CapabilityPolicy {
                web: Some(false),
                code: Some(false),
                mcp: None,
                client: None,
            },
        );
        let grants = CapabilityPolicy {
            web: Some(true),
            code: Some(true),
            mcp: None,
            client: None,
        };
        let caps = resolve_effective(&policies, Some("reader"), Some(&grants))
            .expect("restricted")
            .expect("ok");
        assert!(!caps.web);
        assert!(!caps.code);
    }

    #[test]
    fn grants_can_tighten_web_from_true_to_false() {
        let mut policies = BTreeMap::new();
        policies.insert(
            "writer".into(),
            CapabilityPolicy {
                web: Some(true),
                code: Some(true),
                mcp: None,
                client: None,
            },
        );
        let grants = CapabilityPolicy {
            web: Some(false),
            code: None,
            mcp: None,
            client: None,
        };
        let caps = resolve_effective(&policies, Some("writer"), Some(&grants))
            .expect("restricted")
            .expect("ok");
        assert!(!caps.web);
        assert!(caps.code);
    }

    #[test]
    fn resolve_unknown_policy_fails() {
        let policies = BTreeMap::new();
        let result = resolve_effective(&policies, Some("missing"), None);
        assert!(matches!(result, Some(Err(_))));
    }

    #[test]
    fn unrestricted_claims_skip_resolution() {
        let policies = BTreeMap::new();
        assert!(resolve_effective(&policies, None, None).is_none());
    }

    #[test]
    fn public_capabilities_json_for_restricted_and_open() {
        let caps = EffectiveCapabilities {
            web: false,
            code: false,
            mcp: McpCapability {
                allow: vec!["search_posts".into()],
                domains: vec!["blog".into()],
                max_mode: Some("read".into()),
            },
            client: ClientCapability {
                allow: vec!["ui_*".into()],
            },
        };
        let restricted = caps.to_public_json(true);
        assert_eq!(restricted["restricted"], true);
        assert_eq!(restricted["web"], false);
        assert_eq!(restricted["mcp"]["allow"][0], "search_posts");
        assert_eq!(caps.to_public_json(false), json!({ "restricted": false }));
    }
}
