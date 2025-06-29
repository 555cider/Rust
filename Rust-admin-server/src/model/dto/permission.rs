use crate::model::entity::permission::Permission;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePermissionRequest {
    #[validate(length(min = 1, message = "Code cannot be empty"))]
    pub code: String,
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePermissionRequest {
    #[validate(length(min = 1, message = "Code cannot be empty"))]
    pub code: Option<String>,
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PermissionResponse {
    pub id: Option<i64>,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Permission> for PermissionResponse {
    fn from(p: Permission) -> Self {
        Self {
            id: p.id,
            code: p.code,
            name: p.name,
            description: p.description,
            category: p.category,
            created_at: Utc.from_utc_datetime(&p.created_at),
            updated_at: Utc.from_utc_datetime(&p.updated_at),
        }
    }
}
