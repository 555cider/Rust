use crate::config::auth::user::User;
use crate::{errors::AppError, model::entity::admin_user::AdminUser};
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthRepository {
    pool: Arc<SqlitePool>,
}

impl AuthRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"SELECT
                admin_user.id as "id!",
                admin_user.username as "username!",
                admin_user.password_hash as "password!",
                user_type.name as "role!"
            FROM admin_user
            INNER JOIN user_type ON admin_user.user_type_id = user_type.id
            WHERE admin_user.username = ? AND admin_user.is_active = 1"#,
            username
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(user)
    }

    pub async fn save_refresh_token(
        &self,
        user_id: i64,
        refresh_token: &str,
    ) -> Result<(), AppError> {
        let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
        let naive_expires_at = expires_at.naive_utc();

        sqlx::query!(
            r#"INSERT INTO user_refresh_token (user_id, refresh_token, expires_at, updated_at)
               VALUES (?, ?, ?, CURRENT_TIMESTAMP)
               ON CONFLICT(user_id) DO UPDATE 
               SET refresh_token = excluded.refresh_token, 
                   expires_at = excluded.expires_at,
                   updated_at = CURRENT_TIMESTAMP"#,
            user_id,
            refresh_token,
            naive_expires_at
        )
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_refresh_token(&self, user_id: i64) -> Result<Option<String>, AppError> {
        let token = sqlx::query_scalar!(
            "SELECT refresh_token FROM user_refresh_token WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(token)
    }

    pub async fn create_user(
        &self,
        username: String,
        password_hash: String,
        user_type_id: i64,
        is_active: bool,
    ) -> Result<i64, AppError> {
        let user = sqlx::query_as!(
            AdminUser,
            r#"INSERT INTO admin_user (username, password_hash, user_type_id, is_active, created_at, updated_at)
                VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                RETURNING id as "id!", username as "username!", password_hash as "password_hash!", 
                user_type_id, is_active as "is_active!", last_login_at, 
                created_at as "created_at!", updated_at as "updated_at!""#,
            username,
            password_hash,
            user_type_id,
            is_active
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(user.id)
    }
}
