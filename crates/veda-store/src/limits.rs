use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const OCC_GLOBAL_KEY: &str = "veda:occ:global";
pub const OCC_VISITOR_PREFIX: &str = "veda:occ:v:";
pub const RL_PREFIX: &str = "veda:rl:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OccupancyError {
    Global,
    Visitor,
}

impl OccupancyError {
    pub fn message(&self) -> &'static str {
        "Too many active streams."
    }
}

#[derive(Clone)]
pub struct Occupancy {
    inner: Arc<OccupancyInner>,
}

struct OccupancyInner {
    global_max: usize,
    per_visitor_max: usize,
    backend: LimitBackend,
}

#[derive(Clone)]
enum LimitBackend {
    Memory(MemoryStore),
    Redis(RedisStore),
}

#[derive(Clone, Default)]
struct MemoryStore {
    counters: Arc<Mutex<HashMap<String, i64>>>,
    windows: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

#[derive(Clone)]
struct RedisStore {
    client: redis::Client,
}

impl Occupancy {
    pub fn memory(global_max: usize, per_visitor_max: usize) -> Self {
        Self::with_backend(
            global_max,
            per_visitor_max,
            LimitBackend::Memory(MemoryStore::default()),
        )
    }

    pub fn redis(url: &str, global_max: usize, per_visitor_max: usize) -> Result<Self, String> {
        let store = RedisStore::connect(url)?;
        Ok(Self::with_backend(
            global_max,
            per_visitor_max,
            LimitBackend::Redis(store),
        ))
    }

    pub fn from_config(redis_url: Option<&str>, global_max: usize, per_visitor_max: usize) -> Self {
        if let Some(url) = redis_url.map(str::trim).filter(|value| !value.is_empty()) {
            if let Ok(occupancy) = Self::redis(url, global_max, per_visitor_max) {
                return occupancy;
            }
        }
        Self::memory(global_max, per_visitor_max)
    }

    fn with_backend(global_max: usize, per_visitor_max: usize, backend: LimitBackend) -> Self {
        Self {
            inner: Arc::new(OccupancyInner {
                global_max,
                per_visitor_max,
                backend,
            }),
        }
    }

    pub fn visitor_key(visitor_id: &str) -> String {
        format!("{OCC_VISITOR_PREFIX}{visitor_id}")
    }

    pub fn acquire(&self, visitor_id: &str) -> Result<OccupancyLease, OccupancyError> {
        if self.inner.global_max == 0 && self.inner.per_visitor_max == 0 {
            return Ok(OccupancyLease {
                inner: None,
                visitor_id: visitor_id.to_string(),
            });
        }
        if self.inner.global_max > 0 {
            let next = self.inner.backend.incr(OCC_GLOBAL_KEY);
            if next > self.inner.global_max as i64 {
                self.inner.backend.decr(OCC_GLOBAL_KEY);
                return Err(OccupancyError::Global);
            }
        }
        if self.inner.per_visitor_max > 0 {
            let key = Self::visitor_key(visitor_id);
            let next = self.inner.backend.incr(&key);
            if next > self.inner.per_visitor_max as i64 {
                self.inner.backend.decr(&key);
                if self.inner.global_max > 0 {
                    self.inner.backend.decr(OCC_GLOBAL_KEY);
                }
                return Err(OccupancyError::Visitor);
            }
        }
        Ok(OccupancyLease {
            inner: Some(self.inner.clone()),
            visitor_id: visitor_id.to_string(),
        })
    }
}

pub struct OccupancyLease {
    inner: Option<Arc<OccupancyInner>>,
    visitor_id: String,
}

impl std::fmt::Debug for OccupancyLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccupancyLease")
            .field("visitor_id", &self.visitor_id)
            .field("active", &self.inner.is_some())
            .finish()
    }
}

impl Drop for OccupancyLease {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            if inner.per_visitor_max > 0 {
                inner
                    .backend
                    .decr(&Occupancy::visitor_key(&self.visitor_id));
            }
            if inner.global_max > 0 {
                inner.backend.decr(OCC_GLOBAL_KEY);
            }
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    max: u32,
    window: Duration,
    backend: LimitBackend,
}

impl RateLimiter {
    pub fn memory(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            backend: LimitBackend::Memory(MemoryStore::default()),
        }
    }

    pub fn redis(url: &str, max: u32, window: Duration) -> Result<Self, String> {
        Ok(Self {
            max,
            window,
            backend: LimitBackend::Redis(RedisStore::connect(url)?),
        })
    }

    pub fn from_config(redis_url: Option<&str>, max: u32, window: Duration) -> Self {
        if let Some(url) = redis_url.map(str::trim).filter(|value| !value.is_empty()) {
            if let Ok(limiter) = Self::redis(url, max, window) {
                return limiter;
            }
        }
        Self::memory(max, window)
    }

    pub fn throttle_key(id: &str) -> String {
        format!("{RL_PREFIX}{id}")
    }

    pub fn hit(&self, key: &str) -> Result<(), u64> {
        if self.max == 0 {
            return Ok(());
        }
        self.backend
            .hit(&Self::throttle_key(key), self.max, self.window)
    }
}

pub fn parse_laravel_throttle(value: &str) -> (u32, u64) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return (30, 60);
    }
    let mut parts = trimmed.split(',');
    let max = parts
        .next()
        .and_then(|part| part.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30);
    let minutes = parts
        .next()
        .and_then(|part| part.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1);
    (max, minutes.saturating_mul(60))
}

impl LimitBackend {
    fn incr(&self, key: &str) -> i64 {
        match self {
            Self::Memory(store) => store.incr(key),
            Self::Redis(store) => store.incr(key),
        }
    }

    fn decr(&self, key: &str) -> i64 {
        match self {
            Self::Memory(store) => store.decr(key),
            Self::Redis(store) => store.decr(key),
        }
    }

    fn hit(&self, key: &str, max: u32, window: Duration) -> Result<(), u64> {
        match self {
            Self::Memory(store) => store.hit(key, max, window),
            Self::Redis(store) => store.hit(key, max, window),
        }
    }
}

impl MemoryStore {
    fn incr(&self, key: &str) -> i64 {
        let mut guard = self.counters.lock().expect("occ");
        let slot = guard.entry(key.to_string()).or_insert(0);
        *slot += 1;
        *slot
    }

    fn decr(&self, key: &str) -> i64 {
        let mut guard = self.counters.lock().expect("occ");
        let slot = guard.entry(key.to_string()).or_insert(0);
        *slot = (*slot - 1).max(0);
        *slot
    }

    fn hit(&self, key: &str, max: u32, window: Duration) -> Result<(), u64> {
        let mut guard = self.windows.lock().expect("rl");
        let now = Instant::now();
        let entry = guard.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) >= window {
            *entry = (0, now);
        }
        if entry.0 >= max {
            let elapsed = now.duration_since(entry.1);
            let retry = window.saturating_sub(elapsed).as_secs().max(1);
            return Err(retry);
        }
        entry.0 += 1;
        Ok(())
    }
}

impl RedisStore {
    fn connect(url: &str) -> Result<Self, String> {
        let client = redis::Client::open(url).map_err(|error| error.to_string())?;
        let mut connection = client.get_connection().map_err(|error| error.to_string())?;
        redis::cmd("PING")
            .query::<String>(&mut connection)
            .map_err(|error| error.to_string())?;
        Ok(Self { client })
    }

    fn connection(&self) -> Result<redis::Connection, String> {
        self.client
            .get_connection()
            .map_err(|error| error.to_string())
    }

    fn incr(&self, key: &str) -> i64 {
        let Ok(mut connection) = self.connection() else {
            return i64::MAX;
        };
        redis::cmd("INCR")
            .arg(key)
            .query(&mut connection)
            .unwrap_or(i64::MAX)
    }

    fn decr(&self, key: &str) -> i64 {
        let Ok(mut connection) = self.connection() else {
            return 0;
        };
        let next: i64 = redis::cmd("DECR")
            .arg(key)
            .query(&mut connection)
            .unwrap_or(0);
        if next < 0 {
            let _: Result<(), redis::RedisError> =
                redis::cmd("SET").arg(key).arg(0).query(&mut connection);
            0
        } else {
            next
        }
    }

    fn hit(&self, key: &str, max: u32, window: Duration) -> Result<(), u64> {
        let Ok(mut connection) = self.connection() else {
            return Err(1);
        };
        let count: i64 = redis::cmd("INCR")
            .arg(key)
            .query(&mut connection)
            .unwrap_or(i64::MAX);
        if count == 1 {
            let _: Result<(), redis::RedisError> = redis::cmd("EXPIRE")
                .arg(key)
                .arg(window.as_secs().max(1))
                .query(&mut connection);
        }
        if count > i64::from(max) {
            let ttl: i64 = redis::cmd("TTL")
                .arg(key)
                .query(&mut connection)
                .unwrap_or(1);
            return Err(ttl.max(1) as u64);
        }
        Ok(())
    }
}
