use crate::{
    errors::AppError,
    model::{
        dto::common::ListQueryParams, dto::permission::CreatePermissionRequest,
        dto::permission::PermissionResponse, dto::permission::UpdatePermissionRequest,
        entity::permission::Permission,
    },
};
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct PermissionRepository {
    pool: Arc<SqlitePool>,
}

impl PermissionRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, req: CreatePermissionRequest) -> Result<i64, AppError> {
        let inserted_id = sqlx::query!(
            "INSERT INTO permission (code, name, description, category) VALUES (?, ?, ?, ?) RETURNING id",
            req.code,
            req.name,
            req.description,
            req.category
        )
        .fetch_one(&*self.pool)
        .await?
        .id;

        Ok(inserted_id)
    }

    pub async fn find_all(
        &self,
        query: ListQueryParams,
    ) -> Result<Vec<PermissionResponse>, AppError> {
        let limit = query.limit.unwrap_or(10);
        let page = query.page.unwrap_or(1);
        if limit <= 0 || page <= 0 {
            return Err(AppError::BadRequest(
                "Invalid pagination parameters".to_string(),
            ));
        }
        let offset = (page - 1) * limit;

        let permissions = sqlx::query_as!(
            Permission,
            "SELECT * FROM permission ORDER BY code LIMIT ? OFFSET ?",
            limit,
            offset
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(permissions
            .into_iter()
            .map(PermissionResponse::from)
            .collect())
    }

    pub async fn find_by_id(&self, id: i32) -> Result<PermissionResponse, AppError> {
        tracing::debug!("Looking up permission with ID: {}", id);

        let result = sqlx::query_as!(
            Permission,
            "SELECT id, code, name, description, category, created_at, updated_at FROM permission WHERE id = ?",
            id
        )
        .fetch_optional(&*self.pool)
        .await?;

        match result {
            Some(permission) => {
                tracing::debug!("Found permission: {:?}", permission);
                Ok(permission.into())
            }
            None => {
                tracing::warn!("Permission not found with ID: {}", id);
                Err(AppError::NotFound(format!(
                    "Permission with ID {} not found",
                    id
                )))
            }
        }
    }

    pub async fn update(
        &self,
        id: i32,
        req: UpdatePermissionRequest,
    ) -> Result<PermissionResponse, AppError> {
        // First, get the current permission to check if it exists
        let _ = self.find_by_id(id).await?;

        // Start building the query
        let mut query = "UPDATE permission SET ".to_string();
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(code) = &req.code {
            updates.push("code = ?");
            params.push(code.clone());
        }

        if let Some(name) = &req.name {
            updates.push("name = ?");
            params.push(name.clone());
        }

        if let Some(description) = &req.description {
            updates.push("description = ?");
            params.push(description.clone());
        }

        if let Some(category) = &req.category {
            updates.push("category = ?");
            params.push(category.clone());
        }

        if updates.is_empty() {
            return self.find_by_id(id).await;
        }

        updates.push("updated_at = CURRENT_TIMESTAMP");
        query.push_str(&updates.join(", "));
        query.push_str(" WHERE id = ?");

        // Execute the query with parameters
        let mut query_builder = sqlx::query(&query);

        // Bind parameters in order
        for param in params {
            query_builder = query_builder.bind(param);
        }

        // Bind the id parameter
        query_builder = query_builder.bind(id);

        // Execute the query
        query_builder.execute(&*self.pool).await?;

        // Return the updated permission
        self.find_by_id(id).await
    }

    pub async fn count(&self) -> Result<i64, AppError> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count FROM permission
            "#
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(result)
    }
}
