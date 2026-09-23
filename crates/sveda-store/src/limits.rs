use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
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
    global_max: AtomicUsize,
    per_visitor_max: AtomicUsize,
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
        Self::redis_with_lease(
            client,
            global_max,
            per_visitor_max,
            OCC_LEASE_TTL,
            OCC_HEARTBEAT,
        )
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

    pub fn connect(redis: Option<&RedisClient>, global_max: usize, per_visitor_max: usize) -> Self {
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
                global_max: AtomicUsize::new(global_max),
                per_visitor_max: AtomicUsize::new(per_visitor_max),
                lease_ttl,
                heartbeat,
                backend,
            }),
        }
    }

    pub fn set_limits(&self, global_max: usize, per_visitor_max: usize) {
        self.inner.global_max.store(global_max, Ordering::Relaxed);
        self.inner
            .per_visitor_max
            .store(per_visitor_max, Ordering::Relaxed);
    }

    pub fn visitor_key(visitor_id: &str) -> String {
        format!("{OCC_VISITOR_PREFIX}{visitor_id}")
    }

    pub fn acquire(&self, visitor_id: &str) -> Result<OccupancyLease, OccupancyError> {
        let global_max = self.inner.global_max.load(Ordering::Relaxed);
        let per_visitor_max = self.inner.per_visitor_max.load(Ordering::Relaxed);
        if global_max == 0 && per_visitor_max == 0 {
            return Ok(OccupancyLease {
                inner: None,
                visitor_id: visitor_id.to_string(),
                lease_id: String::new(),
                heartbeat: None,
                counted_global: false,
                counted_visitor: false,
            });
        }
        let lease_id = uuid::Uuid::new_v4().to_string();
        match &self.inner.backend {
            OccupancyBackend::Memory(store) => {
                if global_max > 0 {
                    let next = store.incr(OCC_GLOBAL_KEY);
                    if next > global_max as i64 {
                        store.decr(OCC_GLOBAL_KEY);
                        return Err(OccupancyError::Global);
                    }
                }
                if per_visitor_max > 0 {
                    let key = Self::visitor_key(visitor_id);
                    let next = store.incr(&key);
                    if next > per_visitor_max as i64 {
                        store.decr(&key);
                        if global_max > 0 {
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
                            global_max.to_string(),
                            per_visitor_max.to_string(),
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
            heartbeat: start_heartbeat(
                self.inner.clone(),
                visitor_id.to_string(),
                lease_id.clone(),
                global_max > 0,
                per_visitor_max > 0,
            ),
            inner: Some(self.inner.clone()),
            visitor_id: visitor_id.to_string(),
            lease_id,
            counted_global: global_max > 0,
            counted_visitor: per_visitor_max > 0,
        })
    }
}

fn start_heartbeat(
    inner: Arc<OccupancyInner>,
    visitor_id: String,
    lease_id: String,
    counted_global: bool,
    counted_visitor: bool,
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
                    inner.heartbeat(&lease_id, &visitor_id, counted_global, counted_visitor);
                }
            })
            .abort_handle(),
    )
}

impl OccupancyInner {
    fn heartbeat(
        &self,
        lease_id: &str,
        visitor_id: &str,
        counted_global: bool,
        counted_visitor: bool,
    ) {
        let OccupancyBackend::Redis(client) = &self.backend else {
            return;
        };
        let expire_at = now_secs() + self.lease_ttl.as_secs() as i64;
        if counted_global {
            let _ = client.zadd_xx(OCC_GLOBAL_KEY, expire_at, lease_id);
        }
        if counted_visitor {
            let _ = client.zadd_xx(&Occupancy::visitor_key(visitor_id), expire_at, lease_id);
        }
    }

    fn release(
        &self,
        lease_id: &str,
        visitor_id: &str,
        counted_global: bool,
        counted_visitor: bool,
    ) {
        match &self.backend {
            OccupancyBackend::Memory(store) => {
                if counted_visitor {
                    store.decr(&Occupancy::visitor_key(visitor_id));
                }
                if counted_global {
                    store.decr(OCC_GLOBAL_KEY);
                }
            }
            OccupancyBackend::Redis(client) => {
                if counted_visitor {
                    let _ = client.zrem(&Occupancy::visitor_key(visitor_id), lease_id);
                }
                if counted_global {
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
    counted_global: bool,
    counted_visitor: bool,
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
            inner.release(
                &self.lease_id,
                &self.visitor_id,
                self.counted_global,
                self.counted_visitor,
            );
        }
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    max: Arc<AtomicU32>,
    window_secs: Arc<AtomicU64>,
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
            max: Arc::new(AtomicU32::new(max)),
            window_secs: Arc::new(AtomicU64::new(window.as_secs().max(1))),
            backend: LimitBackend::Memory(MemoryStore::default()),
        }
    }

    pub fn redis(client: RedisClient, max: u32, window: Duration) -> Self {
        Self {
            max: Arc::new(AtomicU32::new(max)),
            window_secs: Arc::new(AtomicU64::new(window.as_secs().max(1))),
            backend: LimitBackend::Redis(client),
        }
    }

    pub fn connect(redis: Option<&RedisClient>, max: u32, window: Duration) -> Self {
        match redis {
            Some(client) => Self::redis(client.clone(), max, window),
            None => Self::memory(max, window),
        }
    }

    pub fn set_limits(&self, max: u32, window: Duration) {
        self.max.store(max, Ordering::Relaxed);
        self.window_secs
            .store(window.as_secs().max(1), Ordering::Relaxed);
    }

    pub fn throttle_key(id: &str) -> String {
        format!("{RL_PREFIX}{id}")
    }

    pub fn hit(&self, key: &str) -> Result<(), u64> {
        let max = self.max.load(Ordering::Relaxed);
        if max == 0 {
            return Ok(());
        }
        let window = Duration::from_secs(self.window_secs.load(Ordering::Relaxed).max(1));
        match &self.backend {
            LimitBackend::Memory(store) => store.hit(&Self::throttle_key(key), max, window),
            LimitBackend::Redis(client) => redis_hit(client, &Self::throttle_key(key), max, window),
        }
    }
}

pub fn parse_laravel_throttle(value: &str) -> (u32, u64) {
    parse_laravel_throttle_default(value, 30)
}

pub fn parse_optional_laravel_throttle(value: &str) -> (u32, u64) {
    if value.trim().is_empty() {
        return (0, 60);
    }
    parse_laravel_throttle_default(value, 0)
}

fn parse_laravel_throttle_default(value: &str, default_max: u32) -> (u32, u64) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return (default_max, 60);
    }
    let mut parts = trimmed.split(',');
    let max_raw = parts.next().unwrap_or("").trim();
    let max = if max_raw == "0" {
        0
    } else {
        max_raw
            .parse::<u32>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(default_max)
    };
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
        let ttl: i64 = redis::cmd("TTL")
            .arg(key)
            .query(&mut connection)
            .unwrap_or(1);
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
