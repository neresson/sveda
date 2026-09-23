use std::time::Duration;

use axum::body::Body;
use axum::http::{HeaderMap, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_protocol::HEADER_ADMIN_KEY;
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;

fn admin_state() -> AppState {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    AppState::new(config)
}

async fn send(state: AppState, method: &str, uri: &str, body: Body) -> (StatusCode, Value) {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert(HEADER_ADMIN_KEY, "sveda-admin-secret".parse().unwrap());
    let mut builder = axum::http::Request::builder().method(method).uri(uri);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let request = builder.body(body).expect("request");
    let response = app(state).oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

#[tokio::test]
async fn local_folder_reaches_ready_with_hashed_embeddings() {
    let dir = std::env::temp_dir().join(format!("sveda-code-index-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(
        dir.join("app/Widget.php"),
        "<?php\nfunction playgroundMarker() { return 1; }\n",
    )
    .unwrap();
    let state = admin_state();
    let (status, created) = send(
        state.clone(),
        "POST",
        "/admin/code-index/store",
        Body::from(
            json!({
                "name": "Playground",
                "provider": "local",
                "local_absolute_path": dir,
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["source"]["status"], "configuring");
    assert_eq!(created["source"]["metadata"]["workspace_ready"], true);
    let id = created["source"]["id"].as_i64().unwrap();

    let (status, scope) = send(
        state.clone(),
        "GET",
        &format!("/admin/code-index/sources/{id}/scope"),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{scope}");
    assert_eq!(scope["ready"], true);
    assert!(scope["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"] == "app" && entry["kind"] == "dir"));

    let (status, started) = send(
        state.clone(),
        "PATCH",
        &format!("/admin/code-index/sources/{id}"),
        Body::from(
            json!({
                "name": "Playground",
                "exclude_paths": [],
                "start_indexing": true
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");

    let mut ready = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let (status, listed) = send(
            state.clone(),
            "GET",
            "/admin/code-index/sources",
            Body::empty(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{listed}");
        let source = &listed["sources"][0];
        if source["status"] == "failed" {
            panic!("indexing failed: {source}");
        }
        if source["status"] == "ready" {
            ready = Some(source.clone());
            break;
        }
    }
    let source = ready.expect("index did not become ready");
    assert!(source["metadata"]["files_indexed"].as_u64().unwrap() >= 1);
    assert!(source["metadata"]["chunks_total"].as_u64().unwrap() >= 1);
    assert_eq!(source["metadata"]["embedding_model"], "local-hash");
    assert_eq!(listed_local_enabled(&state).await, true);
    let _ = std::fs::remove_dir_all(dir);
}

async fn listed_local_enabled(state: &AppState) -> bool {
    let (status, listed) = send(
        state.clone(),
        "GET",
        "/admin/code-index/sources",
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    listed["localIndexingEnabled"].as_bool().unwrap()
}
