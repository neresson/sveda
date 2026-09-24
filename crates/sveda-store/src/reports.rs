use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::postgres::Postgres;
use crate::StoreError;

pub const CONTENT_REPORT_LIST_LIMIT: i64 = 200;

#[derive(Clone, Debug)]
pub struct ContentReport {
    pub id: String,
    pub visitor_id: String,
    pub reason: String,
    pub excerpt: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewContentReport {
    pub visitor_id: String,
    pub reason: String,
    pub excerpt: String,
}

#[derive(Clone)]
pub enum ContentReportStore {
    Memory(MemoryContentReportStore),
    Postgres(Postgres),
}

impl ContentReportStore {
    pub fn memory() -> Self {
        Self::Memory(MemoryContentReportStore::default())
    }

    pub fn postgres(postgres: Postgres) -> Self {
        Self::Postgres(postgres)
    }

    pub async fn insert(&self, report: NewContentReport) -> Result<ContentReport, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.insert(report)),
            Self::Postgres(store) => store.insert_content_report(report).await,
        }
    }

    pub async fn list(&self) -> Result<Vec<ContentReport>, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.list()),
            Self::Postgres(store) => store.list_content_reports().await,
        }
    }
}

#[derive(Clone, Default)]
pub struct MemoryContentReportStore {
    inner: Arc<Mutex<Vec<ContentReport>>>,
}

impl MemoryContentReportStore {
    pub fn insert(&self, report: NewContentReport) -> ContentReport {
        let stored = ContentReport {
            id: Uuid::new_v4().to_string(),
            visitor_id: report.visitor_id,
            reason: report.reason,
            excerpt: report.excerpt,
            created_at: Utc::now(),
        };
        let mut rows = self.inner.lock().expect("content reports");
        rows.push(stored.clone());
        stored
    }

    pub fn list(&self) -> Vec<ContentReport> {
        let mut rows = self.inner.lock().expect("content reports").clone();
        rows.sort_by_key(|row| std::cmp::Reverse(row.created_at));
        rows.truncate(usize::try_from(CONTENT_REPORT_LIST_LIMIT).unwrap_or(200));
        rows
    }
}

impl Postgres {
    pub async fn insert_content_report(
        &self,
        report: NewContentReport,
    ) -> Result<ContentReport, StoreError> {
        let stored = ContentReport {
            id: Uuid::new_v4().to_string(),
            visitor_id: report.visitor_id,
            reason: report.reason,
            excerpt: report.excerpt,
            created_at: Utc::now(),
        };
        sqlx::query(
            r#"
            INSERT INTO content_reports (id, visitor_id, reason, excerpt, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&stored.id)
        .bind(&stored.visitor_id)
        .bind(&stored.reason)
        .bind(&stored.excerpt)
        .bind(stored.created_at)
        .execute(self.pool())
        .await?;
        Ok(stored)
    }

    pub async fn list_content_reports(&self) -> Result<Vec<ContentReport>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, visitor_id, reason, excerpt, created_at
            FROM content_reports
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(CONTENT_REPORT_LIST_LIMIT)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ContentReport {
                id: row.get("id"),
                visitor_id: row.get("visitor_id"),
                reason: row.get("reason"),
                excerpt: row.get("excerpt"),
                created_at: row.get("created_at"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_lists_newest_first() {
        let store = MemoryContentReportStore::default();
        let older = store.insert(NewContentReport {
            visitor_id: "a".into(),
            reason: "hate".into(),
            excerpt: "first".into(),
        });
        let newer = store.insert(NewContentReport {
            visitor_id: "b".into(),
            reason: "violence".into(),
            excerpt: "second".into(),
        });
        store
            .inner
            .lock()
            .expect("reports")
            .iter_mut()
            .for_each(|row| {
                if row.id == older.id {
                    row.created_at = Utc::now() - chrono::Duration::seconds(30);
                }
            });
        let listed = store.list();
        assert_eq!(listed[0].id, newer.id);
        assert_eq!(listed[1].reason, "hate");
    }
}
