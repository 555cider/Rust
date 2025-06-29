use crate::{
    errors::AppError,
    model::{
        dto::history::HistoryListQuery, dto::history::HistoryResponse, entity::history::History,
    },
    repository::history::HistoryRepository,
};
use serde_json::json;
use tracing::info;

/// Service for managing history
#[derive(Clone)]
pub struct HistoryService {
    history_repo: HistoryRepository,
}

impl HistoryService {
    /// Creates a new history service instance
    pub fn new(history_repo: HistoryRepository) -> Self {
        Self { history_repo }
    }

    /// Creates a new history entry
    pub async fn create_log(
        &self,
        user_id: Option<i64>,
        action: impl Into<String>,
        entity_id: Option<i64>,
        details: Option<serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<History, AppError> {
        let action_str = action.into();
        self.history_repo
            .create(
                user_id, action_str, entity_id, details, ip_address, user_agent,
            )
            .await
    }

    /// Helper method to log user login history
    /// Log a successful login attempt
    pub async fn log_login_success(
        &self,
        user_id: i64,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<History, AppError> {
        let details = json!({
            "event": "login_success",
            "ip_address": ip_address,
            "user_agent": user_agent
        });

        self.create_log(
            Some(user_id),
            "user_login",
            None,
            Some(details),
            ip_address,
            user_agent,
        )
        .await
    }

    /// Logs a failed login attempt
    pub async fn log_login_failed(
        &self,
        username: &str,
        reason: String,
        ip_address: Option<String>,
    ) -> Result<History, AppError> {
        let details = json!({ "username": username, "reason": reason });

        self.create_log(None, "login_failed", None, Some(details), ip_address, None)
            .await
    }

    /// Logs user registration
    pub async fn log_user_created(
        &self,
        user_id: i64,
        username: String,
        role_id: i64,
        ip_address: Option<String>,
    ) -> Result<History, AppError> {
        let details = json!({
            "username": username,
            "role_id": role_id,
            "ip_address": ip_address
        });

        self.create_log(
            Some(user_id),
            "user_created",
            Some(user_id),
            Some(details),
            ip_address,
            None,
        )
        .await
    }

    /// Logs token refresh
    pub async fn log_token_refresh(
        &self,
        user_id: i64,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<History, AppError> {
        let details = json!({
            "event": "token_refresh",
            "ip_address": ip_address,
            "user_agent": user_agent
        });

        self.create_log(
            Some(user_id),
            "token_refresh",
            None,
            Some(details),
            ip_address,
            user_agent,
        )
        .await
    }

    /// Retrieves recent history with pagination and filtering
    pub async fn get_recent_history(
        &self,
        query: HistoryListQuery,
    ) -> Result<Vec<HistoryResponse>, AppError> {
        // Get the logs from the repository
        let logs = self.history_repo.list(&query).await?;

        // Convert to response DTOs
        let response = logs.into_iter().map(HistoryResponse::from).collect();

        Ok(response)
    }

    /// Retrieves a history by its ID
    pub async fn get_history_by_id(&self, id: i64) -> Result<Option<HistoryResponse>, AppError> {
        match self.history_repo.find_by_id(id).await? {
            Some(log) => Ok(Some(HistoryResponse::from(log))),
            None => Ok(None),
        }
    }

    /// Counts the number of history entries matching the query
    pub async fn count_history(&self, query: &HistoryListQuery) -> Result<i64, AppError> {
        self.history_repo.count(query).await
    }

    /// Cleans up history older than the specified number of days
    pub async fn cleanup_old_logs(&self, days: i64) -> Result<u64, AppError> {
        if days <= 0 {
            return Err(AppError::BadRequest(
                "Days must be greater than 0".to_string(),
            ));
        }

        info!("Cleaning up history older than {} days", days);
        let deleted = self.history_repo.delete_older_than_days(days).await?;

        if deleted > 0 {
            info!("Deleted {} old history entries", deleted);
        }
        Ok(deleted)
    }
}
