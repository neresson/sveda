use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::postgres::Postgres;
use crate::StoreError;

pub const DASHBOARD_PERIOD_DAYS: u32 = 14;
pub const USAGE_PAGE_SIZE: u32 = 25;

#[derive(Clone, Debug)]
pub struct UsageEvent {
    pub visitor_id: String,
    pub chat_id: String,
    pub model: String,
    pub status: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub tokens_used: u64,
}

#[derive(Clone, Debug)]
pub struct UsageRow {
    pub id: String,
    pub model: String,
    pub tokens_used: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UsageByModel {
    pub model: String,
    pub requests: u64,
    pub tokens_used: u64,
}

#[derive(Clone, Debug)]
pub struct DayBucket {
    pub date: NaiveDate,
    pub requests: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub tokens_used: u64,
    pub unsplit_tokens: u64,
}

fn empty_day(date: NaiveDate) -> DayBucket {
    DayBucket {
        date,
        requests: 0,
        prompt_tokens: 0,
        completion_tokens: 0,
        tokens_used: 0,
        unsplit_tokens: 0,
    }
}

#[derive(Clone, Debug)]
pub struct DashboardStats {
    pub period_days: u32,
    pub requests: u64,
    pub completed: u64,
    pub failed: u64,
    pub pending: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub tokens_used: u64,
    pub unsplit_tokens: u64,
    pub users: u64,
    pub conversations: u64,
    pub avg_tokens: u64,
    pub success_rate: u64,
    pub series: Vec<DayBucket>,
}

#[derive(Clone, Debug)]
pub struct UsageList {
    pub by_model: Vec<UsageByModel>,
    pub rows: Vec<UsageRow>,
    pub current_page: u32,
    pub last_page: u32,
    pub per_page: u32,
    pub total: u64,
}

#[derive(Clone, Debug)]
struct StoredUsage {
    id: String,
    visitor_id: String,
    chat_id: String,
    model: String,
    status: String,
    prompt_tokens: u64,
    completion_tokens: u64,
    tokens_used: u64,
    created_at: DateTime<Utc>,
}

impl From<&UsageEvent> for StoredUsage {
    fn from(event: &UsageEvent) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            visitor_id: event.visitor_id.clone(),
            chat_id: event.chat_id.clone(),
            model: event.model.clone(),
            status: event.status.clone(),
            prompt_tokens: event.prompt_tokens,
            completion_tokens: event.completion_tokens,
            tokens_used: event.tokens_used,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone)]
pub enum UsageStore {
    Memory(MemoryUsageStore),
    Postgres(Postgres),
}

impl UsageStore {
    pub fn memory() -> Self {
        Self::Memory(MemoryUsageStore::new())
    }

    pub fn postgres(postgres: Postgres) -> Self {
        Self::Postgres(postgres)
    }

    pub async fn record(&self, event: UsageEvent) -> Result<(), StoreError> {
        match self {
            Self::Memory(store) => {
                store.record(event);
                Ok(())
            }
            Self::Postgres(store) => store.record_usage(event).await,
        }
    }

    pub async fn dashboard(&self, period_days: u32) -> Result<DashboardStats, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.dashboard(period_days)),
            Self::Postgres(store) => store.usage_dashboard(period_days).await,
        }
    }

    pub async fn page(&self, page: u32, per_page: u32) -> Result<UsageList, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.page(page, per_page)),
            Self::Postgres(store) => store.usage_page(page, per_page).await,
        }
    }
}

#[derive(Clone, Default)]
pub struct MemoryUsageStore {
    inner: Arc<Mutex<Vec<StoredUsage>>>,
}

impl MemoryUsageStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, event: UsageEvent) {
        self.inner
            .lock()
            .expect("usage lock")
            .push(StoredUsage::from(&event));
    }

    pub fn dashboard(&self, period_days: u32) -> DashboardStats {
        let days = period_days.max(1);
        let (since, dates) = period_window(days);
        let rows: Vec<StoredUsage> = self
            .inner
            .lock()
            .expect("usage lock")
            .iter()
            .filter(|row| row.created_at >= since)
            .cloned()
            .collect();
        stats_from_rows(&rows, days, &dates)
    }

    pub fn page(&self, page: u32, per_page: u32) -> UsageList {
        let mut rows: Vec<StoredUsage> = self.inner.lock().expect("usage lock").clone();
        rows.sort_by_key(|row| std::cmp::Reverse(row.created_at));
        paginate(&rows, page, per_page)
    }
}

impl Postgres {
    pub async fn record_usage(&self, event: UsageEvent) -> Result<(), StoreError> {
        let stored = StoredUsage::from(&event);
        sqlx::query(
            r#"
            INSERT INTO usage_requests (
                id, visitor_id, chat_id, model, status,
                prompt_tokens, completion_tokens, tokens_used, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(&stored.id)
        .bind(&stored.visitor_id)
        .bind(&stored.chat_id)
        .bind(&stored.model)
        .bind(&stored.status)
        .bind(i64::try_from(stored.prompt_tokens).unwrap_or(i64::MAX))
        .bind(i64::try_from(stored.completion_tokens).unwrap_or(i64::MAX))
        .bind(i64::try_from(stored.tokens_used).unwrap_or(i64::MAX))
        .bind(stored.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn usage_dashboard(&self, period_days: u32) -> Result<DashboardStats, StoreError> {
        let days = period_days.max(1);
        let (since, dates) = period_window(days);
        let totals = sqlx::query(
            r#"
            SELECT
                COUNT(*)::bigint AS requests,
                COUNT(*) FILTER (WHERE status = 'completed')::bigint AS completed,
                COUNT(*) FILTER (WHERE status = 'failed')::bigint AS failed,
                COUNT(*) FILTER (WHERE status = 'pending')::bigint AS pending,
                COALESCE(SUM(prompt_tokens), 0)::bigint AS prompt_tokens,
                COALESCE(SUM(completion_tokens), 0)::bigint AS completion_tokens,
                COALESCE(SUM(tokens_used), 0)::bigint AS tokens_used,
                COALESCE(SUM(CASE
                    WHEN prompt_tokens = 0 AND completion_tokens = 0 THEN tokens_used
                    ELSE 0
                END), 0)::bigint AS unsplit_tokens,
                COUNT(DISTINCT visitor_id)::bigint AS users,
                COUNT(DISTINCT chat_id)::bigint AS conversations
            FROM usage_requests
            WHERE created_at >= $1
            "#,
        )
        .bind(since)
        .fetch_one(self.pool())
        .await?;

        let day_rows = sqlx::query(
            r#"
            SELECT
                (created_at AT TIME ZONE 'UTC')::date AS day,
                COUNT(*)::bigint AS requests,
                COALESCE(SUM(prompt_tokens), 0)::bigint AS prompt_tokens,
                COALESCE(SUM(completion_tokens), 0)::bigint AS completion_tokens,
                COALESCE(SUM(tokens_used), 0)::bigint AS tokens_used,
                COALESCE(SUM(CASE
                    WHEN prompt_tokens = 0 AND completion_tokens = 0 THEN tokens_used
                    ELSE 0
                END), 0)::bigint AS unsplit_tokens
            FROM usage_requests
            WHERE created_at >= $1
            GROUP BY 1
            "#,
        )
        .bind(since)
        .fetch_all(self.pool())
        .await?;

        let mut by_day: HashMap<NaiveDate, DayBucket> = HashMap::new();
        for row in day_rows {
            let date: NaiveDate = row.try_get("day")?;
            by_day.insert(
                date,
                DayBucket {
                    date,
                    requests: as_u64(row.try_get("requests")?),
                    prompt_tokens: as_u64(row.try_get("prompt_tokens")?),
                    completion_tokens: as_u64(row.try_get("completion_tokens")?),
                    tokens_used: as_u64(row.try_get("tokens_used")?),
                    unsplit_tokens: as_u64(row.try_get("unsplit_tokens")?),
                },
            );
        }

        Ok(finish_stats(
            days,
            as_u64(totals.try_get("requests")?),
            as_u64(totals.try_get("completed")?),
            as_u64(totals.try_get("failed")?),
            as_u64(totals.try_get("pending")?),
            as_u64(totals.try_get("prompt_tokens")?),
            as_u64(totals.try_get("completion_tokens")?),
            as_u64(totals.try_get("tokens_used")?),
            as_u64(totals.try_get("unsplit_tokens")?),
            as_u64(totals.try_get("users")?),
            as_u64(totals.try_get("conversations")?),
            fill_series(&dates, by_day),
        ))
    }

    pub async fn usage_page(&self, page: u32, per_page: u32) -> Result<UsageList, StoreError> {
        let per_page = per_page.max(1);
        let total = as_u64(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM usage_requests")
                .fetch_one(self.pool())
                .await?,
        );
        let last_page = last_page(total, per_page);
        let current = page.max(1).min(last_page);
        let offset = i64::from((current - 1) * per_page);
        let limit = i64::from(per_page);

        let model_rows = sqlx::query(
            r#"
            SELECT
                model,
                COUNT(*)::bigint AS requests,
                COALESCE(SUM(tokens_used), 0)::bigint AS tokens_used
            FROM usage_requests
            GROUP BY model
            ORDER BY tokens_used DESC, model ASC
            "#,
        )
        .fetch_all(self.pool())
        .await?;
        let by_model = model_rows
            .into_iter()
            .map(|row| {
                Ok(UsageByModel {
                    model: row.try_get("model")?,
                    requests: as_u64(row.try_get("requests")?),
                    tokens_used: as_u64(row.try_get("tokens_used")?),
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        let rows = sqlx::query(
            r#"
            SELECT id, model, tokens_used, created_at
            FROM usage_requests
            ORDER BY created_at DESC, id DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;
        let rows = rows
            .into_iter()
            .map(|row| {
                Ok(UsageRow {
                    id: row.try_get("id")?,
                    model: row.try_get("model")?,
                    tokens_used: as_u64(row.try_get("tokens_used")?),
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        Ok(UsageList {
            by_model,
            rows,
            current_page: current,
            last_page,
            per_page,
            total,
        })
    }
}

fn period_window(period_days: u32) -> (DateTime<Utc>, Vec<NaiveDate>) {
    let today = Utc::now().date_naive();
    let start = today - Duration::days(i64::from(period_days.saturating_sub(1)));
    let since = start.and_hms_opt(0, 0, 0).expect("midnight").and_utc();
    let dates = (0..i64::from(period_days))
        .map(|offset| start + Duration::days(offset))
        .collect();
    (since, dates)
}

fn stats_from_rows(rows: &[StoredUsage], period_days: u32, dates: &[NaiveDate]) -> DashboardStats {
    let mut by_day: HashMap<NaiveDate, DayBucket> = HashMap::new();
    let mut visitors = HashMap::<&str, ()>::new();
    let mut chats = HashMap::<&str, ()>::new();
    let mut completed = 0u64;
    let mut failed = 0u64;
    let mut pending = 0u64;
    let mut prompt_tokens = 0u64;
    let mut completion_tokens = 0u64;
    let mut tokens_used = 0u64;
    let mut unsplit_tokens = 0u64;

    for row in rows {
        visitors.insert(&row.visitor_id, ());
        chats.insert(&row.chat_id, ());
        match row.status.as_str() {
            "failed" => failed += 1,
            "pending" => pending += 1,
            _ => completed += 1,
        }
        prompt_tokens += row.prompt_tokens;
        completion_tokens += row.completion_tokens;
        tokens_used += row.tokens_used;
        let unsplit = if row.prompt_tokens == 0 && row.completion_tokens == 0 {
            row.tokens_used
        } else {
            0
        };
        unsplit_tokens += unsplit;
        let date = row.created_at.date_naive();
        let bucket = by_day.entry(date).or_insert_with(|| empty_day(date));
        bucket.date = date;
        bucket.requests += 1;
        bucket.prompt_tokens += row.prompt_tokens;
        bucket.completion_tokens += row.completion_tokens;
        bucket.tokens_used += row.tokens_used;
        bucket.unsplit_tokens += unsplit;
    }

    finish_stats(
        period_days,
        rows.len() as u64,
        completed,
        failed,
        pending,
        prompt_tokens,
        completion_tokens,
        tokens_used,
        unsplit_tokens,
        visitors.len() as u64,
        chats.len() as u64,
        fill_series(dates, by_day),
    )
}

fn finish_stats(
    period_days: u32,
    requests: u64,
    completed: u64,
    failed: u64,
    pending: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    tokens_used: u64,
    unsplit_tokens: u64,
    users: u64,
    conversations: u64,
    series: Vec<DayBucket>,
) -> DashboardStats {
    DashboardStats {
        period_days,
        requests,
        completed,
        failed,
        pending,
        prompt_tokens,
        completion_tokens,
        tokens_used,
        unsplit_tokens,
        users,
        conversations,
        avg_tokens: if completed == 0 {
            0
        } else {
            tokens_used / completed
        },
        success_rate: if requests == 0 {
            0
        } else {
            (completed * 100) / requests
        },
        series,
    }
}

fn fill_series(dates: &[NaiveDate], mut by_day: HashMap<NaiveDate, DayBucket>) -> Vec<DayBucket> {
    dates
        .iter()
        .map(|date| by_day.remove(date).unwrap_or_else(|| empty_day(*date)))
        .collect()
}

fn paginate(rows: &[StoredUsage], page: u32, per_page: u32) -> UsageList {
    let per_page = per_page.max(1);
    let total = rows.len() as u64;
    let last_page = last_page(total, per_page);
    let current = page.max(1).min(last_page);
    let start = ((current - 1) * per_page) as usize;
    let sliced = rows
        .get(start..)
        .unwrap_or(&[])
        .iter()
        .take(per_page as usize);
    let mut by_model: HashMap<String, UsageByModel> = HashMap::new();
    for row in rows {
        let entry = by_model.entry(row.model.clone()).or_insert(UsageByModel {
            model: row.model.clone(),
            requests: 0,
            tokens_used: 0,
        });
        entry.requests += 1;
        entry.tokens_used += row.tokens_used;
    }
    let mut by_model: Vec<UsageByModel> = by_model.into_values().collect();
    by_model.sort_by(|a, b| {
        b.tokens_used
            .cmp(&a.tokens_used)
            .then_with(|| a.model.cmp(&b.model))
    });
    UsageList {
        by_model,
        rows: sliced
            .map(|row| UsageRow {
                id: row.id.clone(),
                model: row.model.clone(),
                tokens_used: row.tokens_used,
                created_at: row.created_at,
            })
            .collect(),
        current_page: current,
        last_page,
        per_page,
        total,
    }
}

fn last_page(total: u64, per_page: u32) -> u32 {
    let per_page = u64::from(per_page.max(1));
    ((total + per_page - 1) / per_page).max(1) as u32
}

fn as_u64(value: i64) -> u64 {
    value.max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_counts_split_tokens_and_fills_empty_days() {
        let store = MemoryUsageStore::new();
        store.record(UsageEvent {
            visitor_id: "a".into(),
            chat_id: "c1".into(),
            model: "flash".into(),
            status: "completed".into(),
            prompt_tokens: 10,
            completion_tokens: 4,
            tokens_used: 14,
        });
        store.record(UsageEvent {
            visitor_id: "b".into(),
            chat_id: "c2".into(),
            model: "flash".into(),
            status: "failed".into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            tokens_used: 3,
        });

        let stats = store.dashboard(14);
        assert_eq!(stats.requests, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.prompt_tokens, 10);
        assert_eq!(stats.completion_tokens, 4);
        assert_eq!(stats.tokens_used, 17);
        assert_eq!(stats.unsplit_tokens, 3);
        assert_eq!(stats.users, 2);
        assert_eq!(stats.conversations, 2);
        assert_eq!(stats.series.len(), 14);
        assert_eq!(stats.series.last().unwrap().requests, 2);
        assert_eq!(stats.success_rate, 50);
    }

    #[test]
    fn usage_page_groups_models() {
        let store = MemoryUsageStore::new();
        store.record(UsageEvent {
            visitor_id: "a".into(),
            chat_id: "c1".into(),
            model: "flash".into(),
            status: "completed".into(),
            prompt_tokens: 2,
            completion_tokens: 1,
            tokens_used: 3,
        });
        let page = store.page(1, 25);
        assert_eq!(page.total, 1);
        assert_eq!(page.by_model.len(), 1);
        assert_eq!(page.by_model[0].model, "flash");
        assert_eq!(page.rows[0].tokens_used, 3);
    }
}
