use crate::errors::AppError;
use axum::{extract::FromRequestParts, http::request::Parts};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug, Clone)]
pub struct AuthnUser {
    pub id: i64,
    pub user_type_id: i64,
    pub username: String,
    pub permissions: Arc<HashSet<String>>,
}

impl<S> FromRequestParts<S> for AuthnUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let authn_user = parts
            .extensions
            .get::<AuthnUser>()
            .ok_or(AppError::Unauthorized("Not authenticated".to_string()))?;
        Ok(authn_user.clone())
    }
}
