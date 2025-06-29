use crate::{
    errors::AppError,
    model::{dto::history::HistoryListQuery, entity::history::History},
};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePool, FromRow, Row};
use std::sync::Arc;
use tracing::error;

/// Internal database representation of a history
#[derive(FromRow)]
struct HistoryDb {
    id: i64,
    user_id: Option<i64>,
    action: String,
    entity_id: Option<i64>,
    details: Option<String>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<HistoryDb> for History {
    fn from(db: HistoryDb) -> Self {
        Self {
            id: db.id,
            user_id: db.user_id,
            action: db.action,
            entity_id: db.entity_id,
            details: db.details,
            ip_address: db.ip_address,
            user_agent: db.user_agent,
            created_at: db.created_at,
        }
    }
}

/// Repository for managing history
#[derive(Clone)]
pub struct HistoryRepository {
    pool: Arc<SqlitePool>,
}

impl HistoryRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Creates a new history entry
    pub async fn create(
        &self,
        user_id: Option<i64>,
        action: String,
        entity_id: Option<i64>,
        details: Option<serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<History, AppError> {
        let details_str = details.as_ref().map(|d| d.to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO history (user_id, action, entity_id, details, ip_address, user_agent)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(action)
        .bind(entity_id)
        .bind(details_str)
        .bind(ip_address)
        .bind(user_agent)
        .map(|row: sqlx::sqlite::SqliteRow| HistoryDb {
            id: row.get("id"),
            user_id: row.get("user_id"),
            action: row.get("action"),
            entity_id: row.get("entity_id"),
            details: row.get("details"),
            ip_address: row.get("ip_address"),
            user_agent: row.get("user_agent"),
            created_at: row.get("created_at"),
        })
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to create history: {}", e);
            AppError::DatabaseError(e)
        })?;

        Ok(result.into())
    }

    /// Retrieves a list of history with pagination and filtering
    pub async fn list(&self, query: &HistoryListQuery) -> Result<Vec<History>, AppError> {
        let (where_clause, _args) = query.to_sql_conditions();
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let query_str = format!(
            "SELECT id, user_id, action, entity_id, details, ip_address, user_agent, created_at 
             FROM history
             {}
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
            where_clause
        );

        // Build and execute the query with parameters
        // The parameters are already bound in the SQL query string with ? placeholders
        // and will be filled in by SQLx when the query is executed
        let query_builder = sqlx::query_as::<_, HistoryDb>(&query_str)
            .bind(limit)
            .bind(offset);

        let logs = query_builder
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to fetch history: {}", e);
                AppError::DatabaseError(e)
            })?
            .into_iter()
            .map(History::from)
            .collect();

        Ok(logs)
    }

    /// Retrieves recent history with pagination
    pub async fn get_recent_history(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<History>, AppError> {
        let logs = sqlx::query_as::<_, HistoryDb>(
            "SELECT id, user_id, action, entity_id, details, ip_address, user_agent, created_at 
             FROM history 
             ORDER BY created_at DESC 
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch recent history: {}", e);
            AppError::InternalServerError("Failed to fetch recent history".to_string())
        })?
        .into_iter()
        .map(History::from)
        .collect();

        Ok(logs)
    }

    /// Finds a history by its ID
    pub async fn find_by_id(&self, id: i64) -> Result<Option<History>, AppError> {
        let log = sqlx::query_as::<_, HistoryDb>(
            "SELECT id, user_id, action, entity_id, details, ip_address, user_agent, created_at 
             FROM history 
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to find history by ID: {}", e);
            AppError::InternalServerError("Failed to find history".to_string())
        })?
        .map(History::from);

        Ok(log)
    }

    /// Deletes history older than the specified number of days
    pub async fn delete_older_than_days(&self, days: i64) -> Result<u64, AppError> {
        let threshold = Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query("DELETE FROM history WHERE created_at < ?")
            .bind(threshold)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to delete old history: {}", e);
                AppError::InternalServerError("Failed to delete old history".to_string())
            })?;

        Ok(result.rows_affected())
    }

    /// Counts the total number of history matching the query (for pagination)
    pub async fn count(&self, query: &HistoryListQuery) -> Result<i64, AppError> {
        let (where_clause, _args) = query.to_sql_conditions();
        let query_str = format!("SELECT COUNT(*) as count FROM history {}", where_clause);

        let count = sqlx::query(&query_str)
            .try_map(|row: sqlx::sqlite::SqliteRow| row.try_get::<i64, _>("count"))
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to count history: {}", e);
                AppError::DatabaseError(e)
            })?;

        Ok(count)
    }
}
