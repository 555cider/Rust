use crate::{
    errors::AppError,
    model::dto::{common::ListQueryParams, user::CreateUserRequest, user::UserResponse},
    repository::user::UserRepository,
    util::password_util,
};
use validator::Validate;

pub struct UserService {
    user_repo: UserRepository,
}

impl UserService {
    pub fn new(user_repo: UserRepository) -> Self {
        Self { user_repo }
    }

    pub async fn create_user(&self, req: CreateUserRequest) -> Result<i64, AppError> {
        req.validate()?;
        let password_hash = password_util::hash_password(&req.password).await?;
        let is_active = req._is_active.unwrap_or(true);

        let user_id = self
            .user_repo
            .create(req.username, password_hash, req.user_type_id, is_active)
            .await?;

        Ok(user_id)
    }

    pub async fn get_user_array(
        &self,
        query_params: ListQueryParams,
    ) -> Result<Vec<UserResponse>, AppError> {
        self.user_repo.find_all(&query_params).await
    }

    pub async fn get_user_by_id(&self, id: i64) -> Result<UserResponse, AppError> {
        self.user_repo.find_by_id(id).await
    }

    pub async fn count_users(&self) -> Result<i64, AppError> {
        self.user_repo.count_users().await
    }

    pub async fn count_active_users(&self) -> Result<i64, AppError> {
        self.user_repo.count_active_users().await
    }
}
