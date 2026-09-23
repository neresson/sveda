use std::time::Duration;

use sveda_mcp::{HostMcpClient, McpCallContext, McpCredentials};

const DEEPWIKI_MCP: &str = "https://mcp.deepwiki.com/mcp";

fn require_public_mcp() -> bool {
    matches!(
        std::env::var("SVEDA_REQUIRE_PUBLIC_MCP")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("yes")
    )
}

fn skip_public_mcp() -> bool {
    matches!(
        std::env::var("SVEDA_SKIP_PUBLIC_MCP")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("yes")
    )
}

#[tokio::test]
async fn deepwiki_public_mcp_lists_documentation_tools() {
    if skip_public_mcp() {
        return;
    }

    let client = HostMcpClient::from_timeout(
        20,
        McpCredentials {
            url: DEEPWIKI_MCP.into(),
            token: String::new(),
        },
        McpCallContext {
            page_context: None,
            chat_id: None,
        },
    );

    let listed = tokio::time::timeout(Duration::from_secs(25), client.list_tools()).await;
    let tools = match listed {
        Ok(Ok(tools)) => tools,
        Ok(Err(error)) if require_public_mcp() => panic!("public MCP failed: {error}"),
        Err(_) if require_public_mcp() => panic!("public MCP timed out"),
        Ok(Err(error)) => {
            eprintln!("skip public MCP (DeepWiki unreachable): {error}");
            return;
        }
        Err(_) => {
            eprintln!("skip public MCP (DeepWiki timed out)");
            return;
        }
    };

    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
    assert!(
        names.contains(&"read_wiki_structure"),
        "missing read_wiki_structure in {names:?}"
    );
    assert!(
        names.contains(&"read_wiki_contents"),
        "missing read_wiki_contents in {names:?}"
    );
    assert!(
        names.contains(&"ask_wiki_question"),
        "missing ask_wiki_question in {names:?}"
    );
}
