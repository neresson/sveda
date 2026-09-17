use std::time::Duration;

use serde_json::json;
use sveda_store::{
    Checkpoint, DocumentStore, HistoryStore, Occupancy, OccupancyError, Postgres, RedisClient,
};

fn test_database_url() -> Option<String> {
    std::env::var("SVEDA_TEST_DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn test_redis_url() -> Option<String> {
    std::env::var("SVEDA_TEST_REDIS_URL")
        .ok()
        .or_else(|| std::env::var("SVEDA_REDIS_URL").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[tokio::test]
async fn postgres_histories_and_settings_roundtrip() {
    let Some(url) = test_database_url() else {
        return;
    };
    let postgres = Postgres::connect(&url).await.expect("postgres");
    postgres.migrate().await.expect("migrate");
    let store = HistoryStore::postgres(postgres.clone());
    let visitor = format!("test-{}", uuid::Uuid::new_v4());
    let chat = "chat-1";

    store
        .checkpoint(Checkpoint {
            visitor_id: visitor.clone(),
            chat_id: chat.into(),
            title: "One".into(),
            preview: "hi".into(),
            messages: vec![json!({"role": "user", "content": "hi"})],
            conversation_history: vec![json!({"role": "user", "content": "hi"})],
            tokens_used: 3,
        })
        .await
        .expect("checkpoint");

    assert_eq!(store.list(&visitor).await.expect("list").len(), 1);
    assert!(store.list("other").await.expect("other").is_empty());
    assert!(store
        .get("other", chat)
        .await
        .expect("get other")
        .is_none());
    assert!(!store
        .rename("other", chat, "Nope")
        .await
        .expect("rename other"));
    assert!(store
        .rename(&visitor, chat, "Two")
        .await
        .expect("rename"));
    let detail = store.get(&visitor, chat).await.expect("get").expect("row");
    assert_eq!(detail.title, "Two");
    assert!(store
        .set_summary(&visitor, chat, "sum".into())
        .await
        .expect("summary"));
    let record = store
        .record(&visitor, chat)
        .await
        .expect("record")
        .expect("exists");
    assert_eq!(record.summary.as_deref(), Some("sum"));
    assert!(store.delete(&visitor, chat).await.expect("delete"));
    assert!(store.get(&visitor, chat).await.expect("gone").is_none());

    let documents = DocumentStore::postgres(postgres);
    let marker = uuid::Uuid::new_v4().to_string();
    documents
        .save(&json!({ "marker": marker }))
        .await
        .expect("save settings");
    let loaded = documents.load().await.expect("load").expect("doc");
    assert_eq!(loaded["marker"], marker);
}

#[tokio::test]
async fn redis_occupancy_drop_and_ttl() {
    let Some(url) = test_redis_url() else {
        return;
    };
    let client = RedisClient::connect(&url).expect("redis");
    let occupancy = Occupancy::redis_with_lease(
        client,
        1,
        0,
        Duration::from_secs(1),
        Duration::ZERO,
    );
    let first = occupancy.acquire("drop-a").expect("first");
    assert_eq!(
        occupancy.acquire("drop-b").unwrap_err(),
        OccupancyError::Global
    );
    drop(first);
    let second = occupancy.acquire("drop-b").expect("after drop");
    drop(second);

    let held = occupancy.acquire("ttl-a").expect("ttl held");
    std::mem::forget(held);
    tokio::time::sleep(Duration::from_millis(1200)).await;
    occupancy.acquire("ttl-b").expect("after ttl");
}

#[tokio::test]
async fn memory_history_store_async_wrapper() {
    let store = HistoryStore::memory();
    store
        .checkpoint(Checkpoint {
            visitor_id: "a".into(),
            chat_id: "chat-1".into(),
            title: "One".into(),
            preview: "hi".into(),
            messages: vec![json!({"role": "user", "content": "hi"})],
            conversation_history: vec![json!({"role": "user", "content": "hi"})],
            tokens_used: 3,
        })
        .await
        .unwrap();
    assert_eq!(store.list("a").await.unwrap().len(), 1);
}
