use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde_json::Value;
use veda_protocol::{HistoryDetail, HistorySummary};

mod limits;
pub use limits::{
    parse_laravel_throttle, Occupancy, OccupancyError, OccupancyLease, RateLimiter, OCC_GLOBAL_KEY,
    OCC_VISITOR_PREFIX, RL_PREFIX,
};

#[derive(Clone, Debug)]
pub struct HistoryRecord {
    pub visitor_id: String,
    pub chat_id: String,
    pub title: String,
    pub preview: String,
    pub messages: Vec<Value>,
    pub conversation_history: Vec<Value>,
    pub tokens_used: u64,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub summary: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub visitor_id: String,
    pub chat_id: String,
    pub title: String,
    pub preview: String,
    pub messages: Vec<Value>,
    pub conversation_history: Vec<Value>,
    pub tokens_used: u64,
}

#[derive(Clone, Default)]
pub struct MemoryHistoryStore {
    inner: Arc<Mutex<HashMap<String, HistoryRecord>>>,
}

impl MemoryHistoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(visitor_id: &str, chat_id: &str) -> String {
        format!("{visitor_id}\0{chat_id}")
    }

    pub fn checkpoint(&self, input: Checkpoint) -> HistoryRecord {
        let mut guard = self.inner.lock().expect("history lock");
        let key = Self::key(&input.visitor_id, &input.chat_id);
        let now = Utc::now();
        let record = if let Some(existing) = guard.get(&key) {
            HistoryRecord {
                visitor_id: input.visitor_id,
                chat_id: input.chat_id,
                title: input.title,
                preview: input.preview,
                messages: input.messages,
                conversation_history: input.conversation_history,
                tokens_used: existing.tokens_used.saturating_add(input.tokens_used),
                version: existing.version.saturating_add(1),
                created_at: existing.created_at,
                updated_at: now,
                summary: existing.summary.clone(),
            }
        } else {
            HistoryRecord {
                visitor_id: input.visitor_id,
                chat_id: input.chat_id,
                title: input.title,
                preview: input.preview,
                messages: input.messages,
                conversation_history: input.conversation_history,
                tokens_used: input.tokens_used,
                version: 1,
                created_at: now,
                updated_at: now,
                summary: None,
            }
        };
        guard.insert(key, record.clone());
        record
    }

    pub fn list(&self, visitor_id: &str) -> Vec<HistorySummary> {
        let mut items: Vec<HistoryRecord> = self
            .inner
            .lock()
            .expect("history lock")
            .values()
            .filter(|record| record.visitor_id == visitor_id)
            .cloned()
            .collect();
        items.sort_by_key(|record| std::cmp::Reverse(record.updated_at));
        items.into_iter().map(summary_of).collect()
    }

    pub fn get(&self, visitor_id: &str, chat_id: &str) -> Option<HistoryDetail> {
        self.record(visitor_id, chat_id).map(detail_of)
    }

    pub fn record(&self, visitor_id: &str, chat_id: &str) -> Option<HistoryRecord> {
        self.inner
            .lock()
            .expect("history lock")
            .get(&Self::key(visitor_id, chat_id))
            .cloned()
    }

    pub fn rename(&self, visitor_id: &str, chat_id: &str, title: &str) -> bool {
        let mut guard = self.inner.lock().expect("history lock");
        let Some(record) = guard.get_mut(&Self::key(visitor_id, chat_id)) else {
            return false;
        };
        record.title = title.to_string();
        record.version = record.version.saturating_add(1);
        record.updated_at = Utc::now();
        true
    }

    pub fn delete(&self, visitor_id: &str, chat_id: &str) -> bool {
        self.inner
            .lock()
            .expect("history lock")
            .remove(&Self::key(visitor_id, chat_id))
            .is_some()
    }

    pub fn set_summary(&self, visitor_id: &str, chat_id: &str, summary: String) -> bool {
        let mut guard = self.inner.lock().expect("history lock");
        let Some(record) = guard.get_mut(&Self::key(visitor_id, chat_id)) else {
            return false;
        };
        record.summary = Some(summary);
        record.updated_at = Utc::now();
        true
    }
}

fn summary_of(record: HistoryRecord) -> HistorySummary {
    HistorySummary {
        id: record.chat_id.clone(),
        chat_id: record.chat_id,
        title: record.title,
        preview: record.preview,
        tokens_used: record.tokens_used,
        version: record.version,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn detail_of(record: HistoryRecord) -> HistoryDetail {
    HistoryDetail {
        id: record.chat_id.clone(),
        chat_id: record.chat_id,
        title: record.title,
        messages: record.messages,
        conversation_history: record.conversation_history,
        tokens_used: record.tokens_used,
        version: record.version,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn checkpoints_are_isolated_by_visitor() {
        let store = MemoryHistoryStore::new();
        store.checkpoint(Checkpoint {
            visitor_id: "a".into(),
            chat_id: "chat-1".into(),
            title: "One".into(),
            preview: "hi".into(),
            messages: vec![json!({"role": "user", "content": "hi"})],
            conversation_history: vec![json!({"role": "user", "content": "hi"})],
            tokens_used: 3,
        });

        assert_eq!(store.list("a").len(), 1);
        assert!(store.list("b").is_empty());
        assert!(store.get("b", "chat-1").is_none());
        assert!(!store.rename("b", "chat-1", "Nope"));
        assert!(store.delete("a", "chat-1"));
        assert!(store.get("a", "chat-1").is_none());
    }
}
