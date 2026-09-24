#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    clippy::manual_checked_ops,
    clippy::manual_div_ceil
)]

mod error;
mod history;
mod kv;
mod limits;
mod postgres;
mod reports;
mod usage;

pub use error::StoreError;
pub use history::{Checkpoint, HistoryRecord, HistoryStore, MemoryHistoryStore};
pub use kv::{mcp_key, KvStore, RedisClient, MCP_PREFIX, SETTINGS_REV_KEY};
pub use limits::{
    parse_laravel_throttle, parse_optional_laravel_throttle, Occupancy, OccupancyError,
    OccupancyLease, RateLimiter, OCC_GLOBAL_KEY, OCC_HEARTBEAT, OCC_LEASE_TTL, OCC_VISITOR_PREFIX,
    RL_PREFIX,
};
pub use postgres::{DocumentStore, Postgres};
pub use reports::{ContentReport, ContentReportStore, NewContentReport};
pub use usage::{
    DashboardStats, DayBucket, UsageByModel, UsageEvent, UsageList, UsageRow, UsageStore,
    DASHBOARD_PERIOD_DAYS, USAGE_PAGE_SIZE,
};
