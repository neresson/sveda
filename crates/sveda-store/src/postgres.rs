use chrono::Utc;
use serde_json::Value;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::Row;
use sveda_protocol::{HistoryDetail, HistorySummary};

use crate::history::{detail_of, summary_of, Checkpoint, HistoryRecord};
use crate::StoreError;

#[derive(Clone)]
pub struct Postgres {
    pool: sqlx::PgPool,
}

impl Postgres {
    pub(crate) fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(url)
            .await?;
        sqlx::query("SELECT 1").execute(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), StoreError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|error| StoreError::message(error.to_string()))
    }

    pub async fn ping(&self) -> Result<(), StoreError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn checkpoint(&self, input: Checkpoint) -> Result<HistoryRecord, StoreError> {
        let messages = Value::Array(input.messages);
        let conversation = Value::Array(input.conversation_history);
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            INSERT INTO histories (
                visitor_id, chat_id, title, preview, messages, conversation_history,
                tokens_used, version, created_at, updated_at, summary
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $8, NULL)
            ON CONFLICT (visitor_id, chat_id) DO UPDATE SET
                title = EXCLUDED.title,
                preview = EXCLUDED.preview,
                messages = EXCLUDED.messages,
                conversation_history = EXCLUDED.conversation_history,
                tokens_used = histories.tokens_used + EXCLUDED.tokens_used,
                version = histories.version + 1,
                updated_at = EXCLUDED.updated_at
            RETURNING visitor_id, chat_id, title, preview, messages, conversation_history,
                      tokens_used, version, created_at, updated_at, summary
            "#,
        )
        .bind(&input.visitor_id)
        .bind(&input.chat_id)
        .bind(&input.title)
        .bind(&input.preview)
        .bind(&messages)
        .bind(&conversation)
        .bind(i64::try_from(input.tokens_used).unwrap_or(i64::MAX))
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        record_from_row(row)
    }

    pub async fn list(&self, visitor_id: &str) -> Result<Vec<HistorySummary>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT visitor_id, chat_id, title, preview, messages, conversation_history,
                   tokens_used, version, created_at, updated_at, summary
            FROM histories
            WHERE visitor_id = $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(visitor_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(record_from_row)
            .map(|record| record.map(summary_of))
            .collect()
    }

    pub async fn get(
        &self,
        visitor_id: &str,
        chat_id: &str,
    ) -> Result<Option<HistoryDetail>, StoreError> {
        Ok(self.record(visitor_id, chat_id).await?.map(detail_of))
    }

    pub async fn record(
        &self,
        visitor_id: &str,
        chat_id: &str,
    ) -> Result<Option<HistoryRecord>, StoreError> {
        let row = sqlx::query(
            r#"
            SELECT visitor_id, chat_id, title, preview, messages, conversation_history,
                   tokens_used, version, created_at, updated_at, summary
            FROM histories
            WHERE visitor_id = $1 AND chat_id = $2
            "#,
        )
        .bind(visitor_id)
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(record_from_row).transpose()
    }

    pub async fn rename(
        &self,
        visitor_id: &str,
        chat_id: &str,
        title: &str,
    ) -> Result<bool, StoreError> {
        let result = sqlx::query(
            r#"
            UPDATE histories
            SET title = $3, version = version + 1, updated_at = $4
            WHERE visitor_id = $1 AND chat_id = $2
            "#,
        )
        .bind(visitor_id)
        .bind(chat_id)
        .bind(title)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete(&self, visitor_id: &str, chat_id: &str) -> Result<bool, StoreError> {
        let result = sqlx::query("DELETE FROM histories WHERE visitor_id = $1 AND chat_id = $2")
            .bind(visitor_id)
            .bind(chat_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn set_summary(
        &self,
        visitor_id: &str,
        chat_id: &str,
        summary: String,
    ) -> Result<bool, StoreError> {
        let result = sqlx::query(
            r#"
            UPDATE histories
            SET summary = $3, updated_at = $4
            WHERE visitor_id = $1 AND chat_id = $2
            "#,
        )
        .bind(visitor_id)
        .bind(chat_id)
        .bind(&summary)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn load_document(&self) -> Result<Option<Value>, StoreError> {
        let row = sqlx::query("SELECT document FROM settings WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(row) => Ok(Some(row.try_get("document")?)),
            None => Ok(None),
        }
    }

    pub async fn save_document(&self, document: &Value) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            INSERT INTO settings (id, document, updated_at)
            VALUES (1, $1, now())
            ON CONFLICT (id) DO UPDATE SET
                document = EXCLUDED.document,
                updated_at = now()
            "#,
        )
        .bind(document)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_code_sources(&self) -> Result<Vec<(i64, Value)>, StoreError> {
        let rows = sqlx::query("SELECT id, document FROM code_sources ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        let mut sources = Vec::with_capacity(rows.len());
        for row in rows {
            sources.push((row.try_get("id")?, row.try_get("document")?));
        }
        Ok(sources)
    }

    pub async fn upsert_code_source(&self, id: i64, document: &Value) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            INSERT INTO code_sources (id, document, updated_at)
            VALUES ($1, $2, now())
            ON CONFLICT (id) DO UPDATE SET
                document = EXCLUDED.document,
                updated_at = now()
            "#,
        )
        .bind(id)
        .bind(document)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_code_source(&self, id: i64) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM code_sources WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct MemoryDocuments {
    inner: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
}

#[derive(Clone)]
pub enum DocumentStore {
    Memory(MemoryDocuments),
    Postgres(Postgres),
}

impl DocumentStore {
    pub fn memory() -> Self {
        Self::Memory(MemoryDocuments::default())
    }

    pub fn postgres(postgres: Postgres) -> Self {
        Self::Postgres(postgres)
    }

    pub fn is_shared(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn load(&self) -> Result<Option<Value>, StoreError> {
        match self {
            Self::Memory(store) => Ok(store.inner.lock().expect("settings docs").clone()),
            Self::Postgres(store) => store.load_document().await,
        }
    }

    pub async fn save(&self, document: &Value) -> Result<(), StoreError> {
        match self {
            Self::Memory(store) => {
                *store.inner.lock().expect("settings docs") = Some(document.clone());
                Ok(())
            }
            Self::Postgres(store) => store.save_document(document).await,
        }
    }
}

fn json_array(value: Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items,
        other => vec![other],
    }
}

fn record_from_row(row: PgRow) -> Result<HistoryRecord, StoreError> {
    Ok(HistoryRecord {
        visitor_id: row.try_get("visitor_id")?,
        chat_id: row.try_get("chat_id")?,
        title: row.try_get("title")?,
        preview: row.try_get("preview")?,
        messages: json_array(row.try_get("messages")?),
        conversation_history: json_array(row.try_get("conversation_history")?),
        tokens_used: row.try_get::<i64, _>("tokens_used")?.max(0) as u64,
        version: row.try_get::<i32, _>("version")?.max(0) as u32,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        summary: row.try_get("summary")?,
    })
}
