use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Default number of items per page for pagination
const DEFAULT_PAGE_SIZE: i64 = 20;
/// Maximum number of items per page
const MAX_PAGE_SIZE: i64 = 100;

/// Response DTO for history entries
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryResponse {
    /// Unique identifier
    pub id: i64,

    /// ID of the user who performed the action (None for system actions)
    pub user_id: Option<i64>,

    /// Username of the user (if available)
    pub username: Option<String>,

    /// Action that was performed (e.g., "login", "create_user")
    pub action: String,

    /// ID of the affected entity (if applicable)
    pub entity_id: Option<i64>,

    /// Type of the affected entity (if applicable)
    pub entity_type: Option<String>,

    /// Additional details about the action
    pub details: Option<Value>,

    /// IP address of the client
    pub ip_address: Option<String>,

    /// Timestamp when the action was performed
    pub created_at: DateTime<Utc>,
}

/// Query parameters for filtering and paginating history
#[derive(Debug, Deserialize, Default, Clone)]
pub struct HistoryListQuery {
    /// Filter by user ID
    pub user_id: Option<i64>,

    /// Filter by action name (e.g., "login", "create_user")
    pub action: Option<String>,

    /// Filter by entity ID
    pub entity_id: Option<i64>,

    /// Filter by entity type (e.g., "user", "permission")
    pub entity_type: Option<String>,

    /// Filter by start date (inclusive)
    pub start_date: Option<DateTime<Utc>>,

    /// Filter by end date (inclusive)
    pub end_date: Option<DateTime<Utc>>,

    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: Option<i64>,

    /// Number of items per page
    #[serde(default = "default_page_size", skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i64>,

    /// Limit for pagination (alternative to per_page)
    pub limit: Option<i64>,

    /// Offset for pagination
    pub offset: Option<i64>,
}

impl HistoryListQuery {
    /// Gets the page number (1-based)
    pub fn get_page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    /// Gets the number of items per page
    pub fn get_limit(&self) -> i64 {
        self.limit
            .or(self.per_page)
            .unwrap_or_else(|| MAX_PAGE_SIZE)
            .clamp(1, i64::MAX)
    }

    /// Calculates the offset for pagination
    pub fn get_offset(&self) -> i64 {
        self.offset
            .unwrap_or_else(|| (self.get_page() - 1) * self.get_limit())
    }

    /// Converts the query into SQL conditions and parameters
    pub fn to_sql_conditions(&self) -> (String, sqlx::sqlite::SqliteArguments) {
        use sqlx::Arguments;

        let mut conditions = Vec::new();
        let mut args = sqlx::sqlite::SqliteArguments::default();
        let mut param_count = 0;

        if let Some(user_id) = self.user_id {
            param_count += 1;
            conditions.push(format!("user_id = ${}", param_count));
            let _ = args.add(user_id);
        }

        if let Some(action) = &self.action {
            param_count += 1;
            conditions.push(format!("action = ${}", param_count));
            let _ = args.add(action);
        }

        if let Some(entity_id) = self.entity_id {
            param_count += 1;
            conditions.push(format!("entity_id = ${}", param_count));
            let _ = args.add(entity_id);
        }

        if let Some(entity_type) = &self.entity_type {
            param_count += 1;
            conditions.push(format!("action LIKE ${}", param_count));
            let _ = args.add(format!("%:{}%", entity_type));
        }

        if let Some(start_date) = self.start_date {
            param_count += 1;
            conditions.push(format!("created_at >= ${}", param_count));
            let _ = args.add(start_date);
        }

        if let Some(end_date) = self.end_date {
            param_count += 1;
            conditions.push(format!("created_at <= ${}", param_count));
            let _ = args.add(end_date);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        (where_clause, args)
    }
}

fn default_page() -> Option<i64> {
    Some(1)
}

fn default_page_size() -> Option<i64> {
    Some(DEFAULT_PAGE_SIZE)
}
