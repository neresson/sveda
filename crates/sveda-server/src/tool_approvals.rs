use std::collections::BTreeMap;
use std::time::Duration;

use chrono::Utc;
use sha2::{Digest, Sha256};
use sveda_store::{KvStore, StoreError};

const TTL: Duration = Duration::from_secs(30 * 60);
const MAX_GRANTS: usize = 32;

pub fn chat_key(visitor: &str, chat: &str) -> String {
    format!("sveda:tool-approvals:{}", fingerprint(&[visitor, chat]))
}

pub fn fingerprint(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    hex::encode(hash.finalize())
}

#[derive(Clone)]
pub struct ChatApprovals {
    kv: KvStore,
    key: String,
    generation: String,
}

impl ChatApprovals {
    pub fn new(kv: KvStore, visitor: &str, chat: &str, generation: i64) -> Self {
        Self {
            kv,
            key: chat_key(visitor, chat),
            generation: generation.to_string(),
        }
    }

    fn read(&self) -> Result<BTreeMap<String, i64>, StoreError> {
        Ok(self
            .kv
            .get_json(&self.key)?
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default())
    }

    fn grant_key(&self, host: &str, tool: Option<&str>, scope: &str) -> String {
        match tool {
            Some(tool) => fingerprint(&[&self.generation, host, tool, scope]),
            None => fingerprint(&[&self.generation, host, scope]),
        }
    }

    pub fn permits(&self, host: &str, tool: Option<&str>, scope: &str) -> bool {
        let grant = self.grant_key(host, tool, scope);
        self.read()
            .ok()
            .and_then(|entries| entries.get(&grant).copied())
            .is_some_and(|expires| expires > Utc::now().timestamp())
    }

    pub fn grant(&self, host: &str, tool: Option<&str>, scope: &str) -> Result<(), StoreError> {
        let now = Utc::now().timestamp();
        let mut entries = self.read()?;
        entries.retain(|_, expires| *expires > now);
        let grant = self.grant_key(host, tool, scope);
        if tool.is_some() {
            let mut legacy = entries.iter().filter(|(_, expires)| **expires != i64::MAX);
            if legacy.clone().count() >= MAX_GRANTS && !entries.contains_key(&grant) {
                if let Some(oldest) = legacy
                    .by_ref()
                    .min_by_key(|(_, expires)| *expires)
                    .map(|(key, _)| key.clone())
                {
                    entries.remove(&oldest);
                }
            }
            entries.insert(grant, now + TTL.as_secs() as i64);
        } else {
            entries.insert(grant, i64::MAX);
        }
        let ttl = (!entries.values().any(|expires| *expires == i64::MAX)).then_some(TTL);
        self.kv
            .set_json(&self.key, &serde_json::json!(entries), ttl)
    }

    pub fn revoke(kv: &KvStore, visitor: &str, chat: &str) -> Result<(), StoreError> {
        kv.set_json(
            &chat_key(visitor, chat),
            &serde_json::Value::Null,
            Some(Duration::from_secs(1)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_chat_approvals_are_isolated_and_revocable() {
        let kv = KvStore::memory();
        let grants = ChatApprovals::new(kv.clone(), "visitor", "chat", 1);
        grants.grant("host", Some("edit"), "org/lesson").unwrap();
        assert!(grants.permits("host", Some("edit"), "org/lesson"));
        assert!(!grants.permits("host", Some("edit"), "other-org/lesson"));
        assert!(!grants.permits("host", Some("publish"), "org/lesson"));
        assert!(!grants.permits("other-host", Some("edit"), "org/lesson"));
        for (visitor, chat, generation) in [
            ("other", "chat", 1),
            ("visitor", "other", 1),
            ("visitor", "chat", 2),
        ] {
            assert!(
                !ChatApprovals::new(kv.clone(), visitor, chat, generation).permits(
                    "host",
                    Some("edit"),
                    "org/lesson"
                )
            );
        }
        let key = fingerprint(&["1", "host", "edit", "org/lesson"]);
        kv.set_json(
            &grants.key,
            &serde_json::json!({ (&key): Utc::now().timestamp() - 1 }),
            None,
        )
        .unwrap();
        assert!(grants.read().unwrap().contains_key(&key));
        assert!(!grants.permits("host", Some("edit"), "org/lesson"));

        grants.grant("host", Some("edit"), "org/lesson").unwrap();
        ChatApprovals::revoke(&kv, "visitor", "chat").unwrap();
        assert!(!grants.permits("host", Some("edit"), "org/lesson"));
    }
}
