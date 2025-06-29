use crate::config::auth::user::User;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<User>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}
