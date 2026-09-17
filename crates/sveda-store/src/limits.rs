use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::kv::RedisClient;

pub const OCC_GLOBAL_KEY: &str = "sveda:occ:global";
pub const OCC_VISITOR_PREFIX: &str = "sveda:occ:v:";
pub const RL_PREFIX: &str = "sveda:rl:";
pub const OCC_LEASE_TTL: Duration = Duration::from_secs(45);
pub const OCC_HEARTBEAT: Duration = Duration::from_secs(15);

const ACQUIRE_SCRIPT: &str = r#"
local global_key = KEYS[1]
local visitor_key = KEYS[2]
local lease_id = ARGV[1]
local now = tonumber(ARGV[2])
local expire_at = tonumber(ARGV[3])
local global_max = tonumber(ARGV[4])
local visitor_max = tonumber(ARGV[5])

redis.call('ZREMRANGEBYSCORE', global_key, '-inf', now)
redis.call('ZREMRANGEBYSCORE', visitor_key, '-inf', now)

if global_max > 0 and redis.call('ZCARD', global_key) >= global_max then
  return 1
end
if visitor_max > 0 and redis.call('ZCARD', visitor_key) >= visitor_max then
  return 2
end
if global_max > 0 then
  redis.call('ZADD', global_key, expire_at, lease_id)
end
if visitor_max > 0 then
  redis.call('ZADD', visitor_key, expire_at, lease_id)
end
return 0
"#;

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
    lease_ttl: Duration,
    heartbeat: Duration,
    backend: OccupancyBackend,
}

#[derive(Clone)]
enum OccupancyBackend {
    Memory(MemoryStore),
    Redis(RedisClient),
}

#[derive(Clone, Default)]
struct MemoryStore {
    counters: Arc<Mutex<HashMap<String, i64>>>,
    windows: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl Occupancy {
    pub fn memory(global_max: usize, per_visitor_max: usize) -> Self {
        Self::with_backend(
            global_max,
            per_visitor_max,
            OCC_LEASE_TTL,
            Duration::ZERO,
            OccupancyBackend::Memory(MemoryStore::default()),
        )
    }

    pub fn redis(client: RedisClient, global_max: usize, per_visitor_max: usize) -> Self {
        Self::redis_with_lease(client, global_max, per_visitor_max, OCC_LEASE_TTL, OCC_HEARTBEAT)
    }

    pub fn redis_with_lease(
        client: RedisClient,
        global_max: usize,
        per_visitor_max: usize,
        lease_ttl: Duration,
        heartbeat: Duration,
    ) -> Self {
        Self::with_backend(
            global_max,
            per_visitor_max,
            lease_ttl,
            heartbeat,
            OccupancyBackend::Redis(client),
        )
    }

    pub fn connect(
        redis: Option<&RedisClient>,
        global_max: usize,
        per_visitor_max: usize,
    ) -> Self {
        match redis {
            Some(client) => Self::redis(client.clone(), global_max, per_visitor_max),
            None => Self::memory(global_max, per_visitor_max),
        }
    }

    fn with_backend(
        global_max: usize,
        per_visitor_max: usize,
        lease_ttl: Duration,
        heartbeat: Duration,
        backend: OccupancyBackend,
    ) -> Self {
        Self {
            inner: Arc::new(OccupancyInner {
                global_max,
                per_visitor_max,
                lease_ttl,
                heartbeat,
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
                lease_id: String::new(),
                heartbeat: None,
            });
        }
        let lease_id = uuid::Uuid::new_v4().to_string();
        match &self.inner.backend {
            OccupancyBackend::Memory(store) => {
                if self.inner.global_max > 0 {
                    let next = store.incr(OCC_GLOBAL_KEY);
                    if next > self.inner.global_max as i64 {
                        store.decr(OCC_GLOBAL_KEY);
                        return Err(OccupancyError::Global);
                    }
                }
                if self.inner.per_visitor_max > 0 {
                    let key = Self::visitor_key(visitor_id);
                    let next = store.incr(&key);
                    if next > self.inner.per_visitor_max as i64 {
                        store.decr(&key);
                        if self.inner.global_max > 0 {
                            store.decr(OCC_GLOBAL_KEY);
                        }
                        return Err(OccupancyError::Visitor);
                    }
                }
            }
            OccupancyBackend::Redis(client) => {
                let now = now_secs();
                let expire_at = now + self.inner.lease_ttl.as_secs() as i64;
                let code = client
                    .eval_i64(
                        ACQUIRE_SCRIPT,
                        &[OCC_GLOBAL_KEY, &Self::visitor_key(visitor_id)],
                        &[
                            lease_id.clone(),
                            now.to_string(),
                            expire_at.to_string(),
                            self.inner.global_max.to_string(),
                            self.inner.per_visitor_max.to_string(),
                        ],
                    )
                    .map_err(|_| OccupancyError::Global)?;
                match code {
                    1 => return Err(OccupancyError::Global),
                    2 => return Err(OccupancyError::Visitor),
                    _ => {}
                }
            }
        }
        Ok(OccupancyLease {
            heartbeat: start_heartbeat(self.inner.clone(), visitor_id.to_string(), lease_id.clone()),
            inner: Some(self.inner.clone()),
            visitor_id: visitor_id.to_string(),
            lease_id,
        })
    }
}

fn start_heartbeat(
    inner: Arc<OccupancyInner>,
    visitor_id: String,
    lease_id: String,
) -> Option<tokio::task::AbortHandle> {
    if inner.heartbeat.is_zero() {
        return None;
    }
    if !matches!(inner.backend, OccupancyBackend::Redis(_)) {
        return None;
    }
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return None;
    };
    let period = inner.heartbeat;
    Some(
        runtime
            .spawn(async move {
                let mut interval = tokio::time::interval(period);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    interval.tick().await;
                    inner.heartbeat(&lease_id, &visitor_id);
                }
            })
            .abort_handle(),
    )
}

impl OccupancyInner {
    fn heartbeat(&self, lease_id: &str, visitor_id: &str) {
        let OccupancyBackend::Redis(client) = &self.backend else {
            return;
        };
        let expire_at = now_secs() + self.lease_ttl.as_secs() as i64;
        if self.global_max > 0 {
            let _ = client.zadd_xx(OCC_GLOBAL_KEY, expire_at, lease_id);
        }
        if self.per_visitor_max > 0 {
            let _ = client.zadd_xx(&Occupancy::visitor_key(visitor_id), expire_at, lease_id);
        }
    }

    fn release(&self, lease_id: &str, visitor_id: &str) {
        match &self.backend {
            OccupancyBackend::Memory(store) => {
                if self.per_visitor_max > 0 {
                    store.decr(&Occupancy::visitor_key(visitor_id));
                }
                if self.global_max > 0 {
                    store.decr(OCC_GLOBAL_KEY);
                }
            }
            OccupancyBackend::Redis(client) => {
                if self.per_visitor_max > 0 {
                    let _ = client.zrem(&Occupancy::visitor_key(visitor_id), lease_id);
                }
                if self.global_max > 0 {
                    let _ = client.zrem(OCC_GLOBAL_KEY, lease_id);
                }
            }
        }
    }
}

pub struct OccupancyLease {
    inner: Option<Arc<OccupancyInner>>,
    visitor_id: String,
    lease_id: String,
    heartbeat: Option<tokio::task::AbortHandle>,
}

impl std::fmt::Debug for OccupancyLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OccupancyLease")
            .field("visitor_id", &self.visitor_id)
            .field("lease_id", &self.lease_id)
            .field("active", &self.inner.is_some())
            .finish()
    }
}

impl Drop for OccupancyLease {
    fn drop(&mut self) {
        if let Some(handle) = self.heartbeat.take() {
            handle.abort();
        }
        if let Some(inner) = self.inner.take() {
            inner.release(&self.lease_id, &self.visitor_id);
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    max: u32,
    window: Duration,
    backend: LimitBackend,
}

#[derive(Clone)]
enum LimitBackend {
    Memory(MemoryStore),
    Redis(RedisClient),
}

impl RateLimiter {
    pub fn memory(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            backend: LimitBackend::Memory(MemoryStore::default()),
        }
    }

    pub fn redis(client: RedisClient, max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            backend: LimitBackend::Redis(client),
        }
    }

    pub fn connect(redis: Option<&RedisClient>, max: u32, window: Duration) -> Self {
        match redis {
            Some(client) => Self::redis(client.clone(), max, window),
            None => Self::memory(max, window),
        }
    }

    pub fn throttle_key(id: &str) -> String {
        format!("{RL_PREFIX}{id}")
    }

    pub fn hit(&self, key: &str) -> Result<(), u64> {
        if self.max == 0 {
            return Ok(());
        }
        match &self.backend {
            LimitBackend::Memory(store) => {
                store.hit(&Self::throttle_key(key), self.max, self.window)
            }
            LimitBackend::Redis(client) => redis_hit(client, &Self::throttle_key(key), self.max, self.window),
        }
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

fn redis_hit(client: &RedisClient, key: &str, max: u32, window: Duration) -> Result<(), u64> {
    let Ok(mut connection) = client.connection() else {
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
        let ttl: i64 = redis::cmd("TTL").arg(key).query(&mut connection).unwrap_or(1);
        return Err(ttl.max(1) as u64);
    }
    Ok(())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}
