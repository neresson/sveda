use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::StoreError;

pub const SETTINGS_REV_KEY: &str = "sveda:settings:rev";
pub const MCP_PREFIX: &str = "sveda:mcp:";

#[derive(Clone)]
pub struct RedisClient {
    client: redis::Client,
}

impl RedisClient {
    pub fn connect(url: &str) -> Result<Self, String> {
        let client = redis::Client::open(url).map_err(|error| error.to_string())?;
        let mut connection = client.get_connection().map_err(|error| error.to_string())?;
        redis::cmd("PING")
            .query::<String>(&mut connection)
            .map_err(|error| error.to_string())?;
        Ok(Self { client })
    }

    pub fn ping(&self) -> Result<(), String> {
        let mut connection = self.connection()?;
        redis::cmd("PING")
            .query::<String>(&mut connection)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn connection(&self) -> Result<redis::Connection, String> {
        self.client
            .get_connection()
            .map_err(|error| error.to_string())
    }

    pub fn incr(&self, key: &str) -> Result<i64, String> {
        let mut connection = self.connection()?;
        redis::cmd("INCR")
            .arg(key)
            .query(&mut connection)
            .map_err(|error| error.to_string())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let mut connection = self.connection()?;
        redis::cmd("GET")
            .arg(key)
            .query(&mut connection)
            .map_err(|error| error.to_string())
    }

    pub fn set_ex(&self, key: &str, value: &str, ttl_secs: u64) -> Result<(), String> {
        let mut connection = self.connection()?;
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl_secs.max(1))
            .query::<String>(&mut connection)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn eval_i64(&self, script: &str, keys: &[&str], args: &[String]) -> Result<i64, String> {
        let mut connection = self.connection()?;
        let mut cmd = redis::cmd("EVAL");
        cmd.arg(script).arg(keys.len());
        for key in keys {
            cmd.arg(*key);
        }
        for arg in args {
            cmd.arg(arg);
        }
        cmd.query(&mut connection)
            .map_err(|error| error.to_string())
    }

    pub fn zadd_xx(&self, key: &str, score: i64, member: &str) -> Result<(), String> {
        let mut connection = self.connection()?;
        redis::cmd("ZADD")
            .arg(key)
            .arg("XX")
            .arg(score)
            .arg(member)
            .query::<i64>(&mut connection)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn zrem(&self, key: &str, member: &str) -> Result<(), String> {
        let mut connection = self.connection()?;
        redis::cmd("ZREM")
            .arg(key)
            .arg(member)
            .query::<i64>(&mut connection)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[derive(Clone)]
pub enum KvStore {
    Memory(MemoryKv),
    Redis(RedisClient),
}

impl KvStore {
    pub fn memory() -> Self {
        Self::Memory(MemoryKv::default())
    }

    pub fn connect(redis: Option<RedisClient>) -> Self {
        match redis {
            Some(client) => Self::Redis(client),
            None => Self::memory(),
        }
    }

    pub fn ping(&self) -> Result<(), StoreError> {
        match self {
            Self::Memory(_) => Ok(()),
            Self::Redis(client) => client.ping().map_err(StoreError::Redis),
        }
    }

    pub fn get_json(&self, key: &str) -> Result<Option<Value>, StoreError> {
        let raw = match self {
            Self::Memory(store) => store.get(key),
            Self::Redis(client) => client.get(key).map_err(StoreError::Redis)?,
        };
        match raw {
            Some(value) => serde_json::from_str(&value)
                .map(Some)
                .map_err(|error| StoreError::message(error.to_string())),
            None => Ok(None),
        }
    }

    pub fn set_json(
        &self,
        key: &str,
        value: &Value,
        ttl: Option<Duration>,
    ) -> Result<(), StoreError> {
        let encoded =
            serde_json::to_string(value).map_err(|error| StoreError::message(error.to_string()))?;
        match self {
            Self::Memory(store) => {
                store.set(key, encoded, ttl);
                Ok(())
            }
            Self::Redis(client) => {
                let ttl_secs = ttl.map(|value| value.as_secs().max(1)).unwrap_or(3600);
                client
                    .set_ex(key, &encoded, ttl_secs)
                    .map_err(StoreError::Redis)
            }
        }
    }

    pub fn settings_rev(&self) -> Result<i64, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.rev()),
            Self::Redis(client) => Ok(client
                .get(SETTINGS_REV_KEY)
                .map_err(StoreError::Redis)?
                .and_then(|value| value.parse().ok())
                .unwrap_or(0)),
        }
    }

    pub fn bump_settings_rev(&self) -> Result<i64, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.bump_rev()),
            Self::Redis(client) => client.incr(SETTINGS_REV_KEY).map_err(StoreError::Redis),
        }
    }
}

#[derive(Clone, Default)]
pub struct MemoryKv {
    values: Arc<Mutex<HashMap<String, (String, Option<Instant>)>>>,
    rev: Arc<Mutex<i64>>,
}

impl MemoryKv {
    fn get(&self, key: &str) -> Option<String> {
        let mut guard = self.values.lock().expect("kv");
        let now = Instant::now();
        match guard.get(key) {
            Some((_, Some(expires))) if *expires <= now => {
                guard.remove(key);
                None
            }
            Some((value, _)) => Some(value.clone()),
            None => None,
        }
    }

    fn set(&self, key: &str, value: String, ttl: Option<Duration>) {
        let expires = ttl.map(|ttl| Instant::now() + ttl);
        self.values
            .lock()
            .expect("kv")
            .insert(key.to_string(), (value, expires));
    }

    fn rev(&self) -> i64 {
        *self.rev.lock().expect("rev")
    }

    fn bump_rev(&self) -> i64 {
        let mut guard = self.rev.lock().expect("rev");
        *guard += 1;
        *guard
    }
}

pub fn mcp_key(visitor_id: &str) -> String {
    format!("{MCP_PREFIX}{visitor_id}")
}
