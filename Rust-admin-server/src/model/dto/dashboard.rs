use crate::model::dto::history::HistoryResponse;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use sqlx::{Row, SqlitePool};

mod ser {
    use serde::{Serialize, Serializer};
    use serde_json::Value;

    #[allow(dead_code)]
    pub fn _serialize<S>(active_percent: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match active_percent {
            Some(s) => {
                let value = serde_json::from_str(s).unwrap_or(Value::Null);
                value.serialize(serializer)
            }
            None => Value::Null.serialize(serializer),
        }
    }

    #[allow(dead_code)]
    pub fn _serialize_option<S>(
        active_percent: &Option<String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match active_percent {
            Some(s) => {
                let value = serde_json::from_str(s).unwrap_or(Value::Null);
                Some(value).serialize(serializer)
            }
            None => None::<Value>.serialize(serializer),
        }
    }
}

#[serde_as]
#[derive(Debug, Serialize)]
pub struct DashboardData {
    // User statistics
    pub total_users: i64,
    pub active_users: i64,
    pub inactive_users: i64,
    pub new_users_today: i64,
    pub new_users_week: i64,

    // System statistics
    pub total_permissions: i64,
    pub total_roles: i64,
    pub total_logs: i64,

    // Activity statistics
    pub recent_history: Vec<HistoryResponse>,
    pub daily_active_users: Vec<DailyActiveUsers>,

    // Calculated fields
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub active_percent: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyActiveUsers {
    pub date: String,
    pub count: i64,
}

impl DashboardData {
    pub async fn new(pool: &SqlitePool, _user_id: i64) -> Result<Self, sqlx::Error> {
        // Get user statistics
        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_user")
            .fetch_one(pool)
            .await?;

        let active_users: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM admin_user WHERE last_login_at >= datetime('now', '-30 days')",
        )
        .fetch_one(pool)
        .await?;

        let inactive_users = total_users - active_users;

        let new_users_today: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM admin_user WHERE created_at >= date('now')")
                .fetch_one(pool)
                .await?;

        let new_users_week: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM admin_user WHERE created_at >= date('now', '-7 days')",
        )
        .fetch_one(pool)
        .await?;

        // Get system statistics
        let total_permissions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permission")
            .fetch_one(pool)
            .await?;

        let total_roles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_type")
            .fetch_one(pool)
            .await?;

        let total_logs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM history")
            .fetch_one(pool)
            .await?;

        // Get daily active users for the last 30 days
        let daily_active_users = sqlx::query_as::<_, DailyActiveUsers>(
            r#"
            SELECT 
                date(created_at) as date, 
                COUNT(DISTINCT user_id) as count 
            FROM history 
            WHERE created_at >= date('now', '-30 days')
            GROUP BY date(created_at)
            ORDER BY date ASC
            "#,
        )
        .fetch_all(pool)
        .await?;
        // Get recent history using raw SQL to avoid type issues with query_as!
        let recent_history_rows = sqlx::query(
            r#"
            SELECT h.id, h.user_id, au.username, h.action, h.entity_id, h.details, h.ip_address, h.created_at 
            FROM history h
            LEFT JOIN admin_user au ON h.user_id = au.id
            ORDER BY h.created_at DESC 
            LIMIT 10"#
        )
        .fetch_all(pool)
        .await?;

        // Convert rows to HistoryResponse
        let mut recent_history = Vec::new();
        for row in recent_history_rows {
            let history = HistoryResponse {
                id: row.get("id"),
                user_id: row.get("user_id"),
                username: row.get("username"),
                action: row.get("action"),
                entity_id: row.get("entity_id"),
                entity_type: None,
                details: row.get("details"),
                ip_address: row.get("ip_address"),
                created_at: row.get("created_at"),
            };
            recent_history.push(history);
        }

        // Calculate active percentage with 2 decimal places
        let active_percent = if total_users > 0 {
            let percent = (active_users as f64 / total_users as f64) * 100.0;
            Some(format!("{:.2}", percent))
        } else {
            None
        };

        Ok(Self {
            // User statistics
            total_users,
            active_users,
            inactive_users,
            new_users_today,
            new_users_week,
            // System statistics
            total_permissions,
            total_roles,
            total_logs,
            // Activity data
            recent_history,
            daily_active_users,
            // Calculated fields
            active_percent,
        })
    }

    pub async fn with_range(
        pool: &SqlitePool,
        _user_id: i64,
        range: &str,
    ) -> Result<Self, sqlx::Error> {
        // Get user statistics (these don't depend on the time range)
        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_user")
            .fetch_one(pool)
            .await?;

        // Determine the time range for active users
        let active_users_sql = match range {
            "week" => {
                "SELECT COUNT(*) FROM admin_user WHERE last_login_at >= datetime('now', '-7 days')"
            }
            "month" => {
                "SELECT COUNT(*) FROM admin_user WHERE last_login_at >= datetime('now', '-30 days')"
            }
            _ => "SELECT COUNT(*) FROM admin_user WHERE last_login_at >= date('now')", // default to day
        };

        let active_users: i64 = sqlx::query_scalar(active_users_sql).fetch_one(pool).await?;

        let inactive_users = total_users - active_users;

        // New users in the selected range
        let new_users_sql = match range {
            "week" => "SELECT COUNT(*) FROM admin_user WHERE created_at >= date('now', '-7 days')",
            "month" => {
                "SELECT COUNT(*) FROM admin_user WHERE created_at >= date('now', '-30 days')"
            }
            _ => "SELECT COUNT(*) FROM admin_user WHERE created_at >= date('now')", // default to day
        };

        let new_users = sqlx::query_scalar(new_users_sql).fetch_one(pool).await?;

        // Get system statistics (these don't depend on time range)
        let total_permissions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM permission")
            .fetch_one(pool)
            .await?;

        let total_roles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_type")
            .fetch_one(pool)
            .await?;

        // Get total logs in the selected range
        let total_logs_sql = match range {
            "week" => "SELECT COUNT(*) FROM history WHERE created_at >= datetime('now', '-7 days')",
            "month" => {
                "SELECT COUNT(*) FROM history WHERE created_at >= datetime('now', '-30 days')"
            }
            _ => "SELECT COUNT(*) FROM history WHERE created_at >= date('now')", // default to day
        };

        let total_logs: i64 = sqlx::query_scalar(total_logs_sql).fetch_one(pool).await?;

        // Get daily active users for the selected range
        let daily_active_users_sql = match range {
            "week" => {
                r#"
                SELECT 
                    date(created_at) as date, 
                    COUNT(DISTINCT user_id) as count 
                FROM history 
                WHERE created_at >= datetime('now', '-7 days')
                GROUP BY date(created_at)
                ORDER BY date ASC"#
            }
            "month" => {
                r#"
                SELECT 
                    date(created_at) as date, 
                    COUNT(DISTINCT user_id) as count 
                FROM history 
                WHERE created_at >= datetime('now', '-30 days')
                GROUP BY date(created_at)
                ORDER BY date ASC"#
            }
            _ => {
                r#"
                SELECT 
                    date(created_at) as date, 
                    COUNT(DISTINCT user_id) as count 
                FROM history 
                WHERE created_at >= date('now')
                GROUP BY date(created_at)
                ORDER BY date ASC"#
            } // default to day
        };

        let daily_active_users = sqlx::query_as::<_, DailyActiveUsers>(daily_active_users_sql)
            .fetch_all(pool)
            .await?;

        // Get recent history (limited to 10 most recent)
        let recent_history_rows = sqlx::query(
            r#"
            SELECT h.id, h.user_id, au.username, h.action, h.entity_id, h.details, h.ip_address, h.created_at 
            FROM history h
            LEFT JOIN admin_user au ON h.user_id = au.id
            ORDER BY h.created_at DESC 
            LIMIT 10"#
        )
        .fetch_all(pool)
        .await?;

        // Convert rows to HistoryResponse
        let mut recent_history = Vec::new();
        for row in recent_history_rows {
            let history = HistoryResponse {
                id: row.get("id"),
                user_id: row.get("user_id"),
                username: row.get("username"),
                action: row.get("action"),
                entity_id: row.get("entity_id"),
                entity_type: None,
                details: row.get("details"),
                ip_address: row.get("ip_address"),
                created_at: row.get("created_at"),
            };
            recent_history.push(history);
        }

        // Calculate active percentage with 2 decimal places
        let active_percent = if total_users > 0 {
            let percent = (active_users as f64 / total_users as f64) * 100.0;
            Some(format!("{:.2}", percent))
        } else {
            None
        };

        Ok(Self {
            // User statistics
            total_users,
            active_users,
            inactive_users,
            new_users_today: new_users,
            new_users_week: new_users, // In this context, we're reusing new_users for both today and week

            // System statistics
            total_permissions,
            total_roles,
            total_logs,

            // Activity data
            recent_history,
            daily_active_users,

            // Calculated fields
            active_percent,
        })
    }
}
