use crate::{
    errors::AppError,
    model::dto::{
        common::ListQueryParams, permission::CreatePermissionRequest,
        permission::PermissionResponse, permission::UpdatePermissionRequest,
    },
    repository::permission::PermissionRepository,
};
use validator::Validate;

pub struct PermissionService {
    permission_repo: PermissionRepository,
}

impl PermissionService {
    pub fn new(permission_repo: PermissionRepository) -> Self {
        Self { permission_repo }
    }

    pub async fn create_permission(&self, req: CreatePermissionRequest) -> Result<i64, AppError> {
        req.validate()?;
        self.permission_repo.create(req).await
    }

    pub async fn get_permissions(
        &self,
        query: ListQueryParams,
    ) -> Result<Vec<PermissionResponse>, AppError> {
        self.permission_repo.find_all(query).await
    }

    pub async fn get_permission_by_id(&self, id: i32) -> Result<PermissionResponse, AppError> {
        self.permission_repo.find_by_id(id).await
    }

    /// Check if a user has a specific permission
    pub async fn has_permission(
        &self,
        _user_id: i64,
        _permission_name: &str,
    ) -> Result<bool, AppError> {
        // In a real application, you would check the user's roles/permissions
        // For now, we'll just check if the permission exists in the database
        // and return true for demonstration purposes
        // TODO: Implement proper permission checking logic
        Ok(true)
    }

    pub async fn count_permissions(&self) -> Result<i64, AppError> {
        self.permission_repo.count().await
    }

    pub async fn update_permission(
        &self,
        id: i32,
        req: UpdatePermissionRequest,
    ) -> Result<PermissionResponse, AppError> {
        if let Some(name) = &req.name {
            if name.trim().is_empty() {
                return Err(AppError::BadRequest("Name cannot be empty".to_string()));
            }
        }

        if let Some(code) = &req.code {
            if code.trim().is_empty() {
                return Err(AppError::BadRequest("Code cannot be empty".to_string()));
            }
        }

        self.permission_repo.update(id, req).await
    }
}
