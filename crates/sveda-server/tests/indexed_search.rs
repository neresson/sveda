use std::sync::Arc;

use serde_json::json;
use sveda_agent::{ToolRuntime, READ_CODE_TOOL_NAME, SEARCH_CODE_TOOL_NAME};

#[tokio::test]
async fn source_index_search_and_read_find_the_file() {
    let dir = std::env::temp_dir().join(format!("sveda-search-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(dir.join("app/Http/Controllers")).unwrap();
    std::fs::write(
        dir.join("app/Http/Controllers/SvedaAccessController.php"),
        "<?php\nclass SvedaAccessController extends Controller {}\n",
    )
    .unwrap();
    let index = Arc::new(sveda_index::prepare(&dir));
    let runtime = ToolRuntime::default().with_source_indexes(vec![index]);
    assert!(runtime
        .advertised()
        .iter()
        .any(|tool| tool.name == SEARCH_CODE_TOOL_NAME));

    let outcome = runtime
        .execute(
            SEARCH_CODE_TOOL_NAME,
            json!({ "query": "SvedaAccessController", "limit": 5 }),
        )
        .await;
    let hits = outcome.output["data"]["hits"].as_array().expect("hits");
    assert!(
        hits.iter().any(|hit| hit["path"]
            .as_str()
            .unwrap_or("")
            .contains("SvedaAccessController.php")),
        "hits: {hits:?}"
    );

    let read = runtime
        .execute(
            READ_CODE_TOOL_NAME,
            json!({
                "path": "app/Http/Controllers/SvedaAccessController.php",
                "length": 800
            }),
        )
        .await;
    let content = read.output["data"]["content"].as_str().unwrap_or("");
    assert!(
        content.contains("class SvedaAccessController"),
        "content: {content}"
    );
    let _ = std::fs::remove_dir_all(dir);
}
