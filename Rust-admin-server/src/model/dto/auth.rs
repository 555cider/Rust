use crate::model::dto::user_type::UserTypeResponse;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "Username cannot be empty"))]
    pub username: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
    pub redirect_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
}

// 현재 로그인한 사용자 정보 응답 DTO
#[derive(Debug, Serialize)]
pub struct CurrentUserResponse {
    pub id: i64,
    pub username: String,
    pub user_type_id: i64,
    pub user_type: Option<UserTypeResponse>, // 사용자 종류 정보 포함 가능
    pub permissions: Vec<String>,            // 사용자 종류에 부여된 권한 코드 목록
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 1, message = "Username cannot be empty"))]
    pub username: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
    #[validate(range(min = 1, message = "User type cannot be empty"))]
    pub user_type_id: i64,
    pub is_active: Option<bool>,
}
