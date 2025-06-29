use crate::{
    errors::AppError,
    model::dto::{
        common::ListQueryParams,
        user_type::{CreateUserTypeRequest, UpdateUserTypeRequest, UserTypeResponse},
    },
    repository::user_type::UserTypeRepository,
};
use std::sync::Arc;
use validator::Validate;

#[derive(Clone)]
pub struct UserTypeService {
    user_type_repo: Arc<UserTypeRepository>,
}

impl UserTypeService {
    pub fn new(user_type_repo: UserTypeRepository) -> Self {
        Self {
            user_type_repo: Arc::new(user_type_repo),
        }
    }

    pub async fn create_user_type(
        &self,
        req: CreateUserTypeRequest,
    ) -> Result<UserTypeResponse, AppError> {
        req.validate()?;

        // Check if code already exists
        let exists = self.user_type_repo.find_by_code(&req.code).await?;
        if exists.is_some() {
            return Err(AppError::Conflict(
                "User type with this code already exists".to_string(),
            ));
        }

        self.user_type_repo.create(req).await
    }

    pub async fn get_user_type_array(
        &self,
        query: ListQueryParams,
    ) -> Result<Vec<UserTypeResponse>, AppError> {
        self.user_type_repo.find_all(query).await
    }

    pub async fn get_user_type_by_id(&self, type_id: i64) -> Result<UserTypeResponse, AppError> {
        self.user_type_repo.find_by_id(type_id).await
    }

    pub async fn update_user_type(
        &self,
        type_id: i64,
        req: UpdateUserTypeRequest,
    ) -> Result<UserTypeResponse, AppError> {
        req.validate()?;

        // If code is being updated, check if it already exists
        if let Some(code) = &req.code {
            if let Some(existing) = self.user_type_repo.find_by_code(code).await? {
                if existing.id != type_id {
                    return Err(AppError::Conflict(
                        "User type with this code already exists".to_string(),
                    ));
                }
            }
        }

        self.user_type_repo.update(type_id, req).await
    }

    pub async fn delete_user_type(&self, type_id: i64) -> Result<(), AppError> {
        self.user_type_repo.delete(type_id).await
    }
}
