use crate::{
    errors::AppError,
    model::{
        dto::{
            common::ListQueryParams,
            user_type::{CreateUserTypeRequest, UpdateUserTypeRequest, UserTypeResponse},
        },
        entity::user_type::UserType,
    },
};
// Remove unused imports
use sqlx::{sqlite::SqliteArguments, Arguments, SqlitePool};
use std::sync::Arc;

#[derive(Clone)]
pub struct UserTypeRepository {
    pool: Arc<SqlitePool>,
}

impl UserTypeRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, req: CreateUserTypeRequest) -> Result<UserTypeResponse, AppError> {
        // First check if a user type with this code already exists
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM user_type WHERE code = ?")
            .bind(&req.code)
            .fetch_optional(&*self.pool)
            .await?;

        if exists.is_some() {
            return Err(AppError::Conflict(
                "User type with this code already exists".to_string(),
            ));
        }

        // Insert the new user type
        let result = sqlx::query!(
            r#"
            INSERT INTO user_type (code, name, description, is_active)
            VALUES (?, ?, ?, ?)
            "#,
            req.code,
            req.name,
            req.description,
            req.is_active
        )
        .execute(&*self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InternalServerError(
                "Failed to create user type".to_string(),
            ));
        }

        // Get the ID of the newly created user type
        let id = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(&*self.pool)
            .await?;

        // Return the full user type
        self.find_by_id(id).await
    }

    pub async fn find_all(
        &self,
        query: ListQueryParams,
    ) -> Result<Vec<UserTypeResponse>, AppError> {
        let limit = query.get_limit();
        let offset = query.get_offset();
        let allowed_sort_columns = [
            "id",
            "code",
            "name",
            "is_active",
            "created_at",
            "updated_at",
        ];
        let order_by = query.get_order_by(&allowed_sort_columns);

        let base_query = "SELECT * FROM user_type WHERE is_active = true";

        let (query_str, params) = if let Some(search) = &query.q {
            let search_term = format!("%{}%", search);
            let query_str = format!(
                "{} AND (code LIKE ? OR name LIKE ? OR description LIKE ?) ORDER BY {} LIMIT ? OFFSET ?",
                base_query, order_by
            );
            (
                query_str,
                vec![
                    search_term.clone(),
                    search_term.clone(),
                    search_term,
                    limit.to_string(),
                    offset.to_string(),
                ],
            )
        } else {
            let query_str = format!("{} ORDER BY {} LIMIT ? OFFSET ?", base_query, order_by);
            (query_str, vec![limit.to_string(), offset.to_string()])
        };

        let mut query_builder = sqlx::query_as::<_, UserType>(&query_str);

        // Add parameters to the query
        for param in params {
            query_builder = query_builder.bind(param);
        }

        let user_types = query_builder
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(UserTypeResponse::from)
            .collect();

        Ok(user_types)
    }

    pub async fn find_by_id(&self, type_id: i64) -> Result<UserTypeResponse, AppError> {
        let user_type = sqlx::query_as::<_, UserType>(
            "SELECT id, code, name, description, is_active, created_at, updated_at FROM user_type WHERE id = ?",
        )
        .bind(type_id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User type not found".to_string()))?;

        Ok(UserTypeResponse::from(user_type))
    }

    pub async fn update(
        &self,
        type_id: i64,
        req: UpdateUserTypeRequest,
    ) -> Result<UserTypeResponse, AppError> {
        let mut updates = Vec::new();
        let mut params = SqliteArguments::default();

        if let Some(code) = req.code {
            updates.push("code = ?");
            let _ = params.add(code);
        }

        if let Some(name) = req.name {
            updates.push("name = ?");
            let _ = params.add(name);
        }

        if let Some(description) = req.description {
            updates.push("description = ?");
            let _ = params.add(description);
        }

        if let Some(is_active) = req.is_active {
            updates.push("is_active = ?");
            let _ = params.add(is_active);
        }

        if updates.is_empty() {
            return self.find_by_id(type_id).await;
        }

        updates.push("updated_at = CURRENT_TIMESTAMP");

        let set_clause = updates.join(", ");
        let query_str = format!("UPDATE user_type SET {} WHERE id = ?", set_clause);

        let _ = params.add(type_id);

        sqlx::query_with(&query_str, params)
            .execute(&*self.pool)
            .await?;

        self.find_by_id(type_id).await
    }

    pub async fn delete(&self, type_id: i64) -> Result<(), AppError> {
        // Check if there are any active users with this user type
        let user_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM admin_user WHERE user_type_id = ? AND is_active = true",
        )
        .bind(type_id)
        .fetch_one(&*self.pool)
        .await?;

        if user_count > 0 {
            return Err(AppError::BadRequest(
                "Cannot deactivate user type that is in use by active users".to_string(),
            ));
        }

        // Soft delete by setting is_active to false
        let result = sqlx::query!(
            "UPDATE user_type SET is_active = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            type_id
        )
        .execute(&*self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("User type not found".to_string()));
        }

        Ok(())
    }

    pub async fn get_user_type_info(
        &self,
        user_type_id: i64,
    ) -> Result<Option<UserTypeResponse>, AppError> {
        let user_type = sqlx::query_as::<_, UserType>(
            "SELECT id, code, name, description, is_active, created_at, updated_at FROM user_type WHERE id = ?",
        )
        .bind(user_type_id)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(user_type.map(UserTypeResponse::from))
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Option<UserTypeResponse>, AppError> {
        let user_type = sqlx::query_as::<_, UserType>(
            "SELECT id, code, name, description, is_active, created_at, updated_at FROM user_type WHERE code = ?",
        )
        .bind(code)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(user_type.map(UserTypeResponse::from))
    }
}
