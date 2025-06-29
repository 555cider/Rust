use crate::model::dto::history::HistoryResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// History entry representing user actions in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    /// Unique identifier
    pub id: i64,

    /// ID of the user who performed the action (None for system actions)
    pub user_id: Option<i64>,

    /// Action name (e.g., "login", "create_user", "update_permission")
    pub action: String,

    /// ID of the affected entity (if applicable)
    pub entity_id: Option<i64>,

    /// Additional details about the action (stored as JSON string)
    pub details: Option<String>,

    /// IP address of the client
    pub ip_address: Option<String>,

    /// User agent of the client
    pub user_agent: Option<String>,

    /// Timestamp when the action was performed
    pub created_at: DateTime<Utc>,
}

impl From<History> for HistoryResponse {
    fn from(log: History) -> Self {
        Self {
            id: log.id,
            user_id: log.user_id,
            username: None, // Will be populated if needed
            action: log.action,
            entity_id: log.entity_id,
            entity_type: None, // Will be populated if needed
            details: log.details.and_then(|d| serde_json::from_str(&d).ok()),
            ip_address: log.ip_address,
            created_at: log.created_at,
        }
    }
}

impl History {
    /// Creates a new history entry
    pub fn new(
        user_id: Option<i64>,
        action: impl Into<String>,
        entity_id: Option<i64>,
        details: Option<Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            id: 0, // Will be set by the database
            user_id,
            action: action.into(),
            entity_id,
            details: details.map(|v| v.to_string()),
            ip_address,
            user_agent,
            created_at: Utc::now(),
        }
    }

    /// Creates a system-generated history entry
    pub fn system(
        action: impl Into<String>,
        entity_id: Option<i64>,
        details: Option<Value>,
    ) -> Self {
        Self::new(None, action, entity_id, details, None, None)
    }
}
