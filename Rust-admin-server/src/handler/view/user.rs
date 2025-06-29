use crate::{
    filter::{auth, UserId},
    model::dto::common::ListQueryParams,
    AppState,
};
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::error::Error;
use tera::Context;
use tracing::error;
use validator::Validate;

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/", get(users_page))
        .route("/new", get(user_create_page))
        .route("/edit/{id}", get(user_edit_page))
}

async fn users_page(
    State(config): State<AppState>,
    Extension(_user_id): Extension<UserId>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "사용자 관리");
    context.insert("active_page", "users");
    context.insert("user_id", &_user_id.0);

    // Add current user info for the template
    if let Ok(current_user) = config.service.user.get_user_by_id(_user_id.0).await {
        context.insert("current_user", &current_user);
    }

    match config.tera.render("user.html", &context) {
        Ok(s) => Html(s).into_response(),
        Err(e) => {
            // Log the full error chain for better debugging
            let mut error_chain = String::new();
            let mut source = e.source();
            error_chain.push_str(&format!("Error: {}", e));

            while let Some(err) = source {
                error_chain.push_str(&format!("\nCaused by: {}", err));
                source = err.source();
            }

            error!(
                "Template rendering error: {}\nContext: {:#?}",
                error_chain, context
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template rendering error: {}", e),
            )
                .into_response()
        }
    }
}

async fn user_create_page(
    State(config): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "사용자 추가");
    context.insert("active_page", "users");
    context.insert("user_id", &user_id.0);

    // Add current user info for the template
    if let Ok(current_user) = config.service.user.get_user_by_id(user_id.0).await {
        context.insert("current_user", &current_user);
    }

    // Get user types for the form
    match config
        .service
        .user_type
        .get_user_type_array(ListQueryParams::default())
        .await
    {
        Ok(user_types) => {
            context.insert("user_types", &user_types);
        }
        Err(e) => {
            error!("Failed to load user types: {}", e);
            context.insert("error", "사용자 유형을 불러오는 중 오류가 발생했습니다.");
        }
    }

    match config.tera.render("user_form.html", &context) {
        Ok(s) => Html(s).into_response(),
        Err(e) => {
            error!("Template rendering error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Template rendering error",
            )
                .into_response()
        }
    }
}

async fn user_edit_page(
    State(config): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "사용자 수정");
    context.insert("active_page", "users");
    context.insert("user_id", &user_id.0);

    // Add current user info for the template
    if let Ok(current_user) = config.service.user.get_user_by_id(user_id.0).await {
        context.insert("current_user", &current_user);
    }

    // Get user data
    match config.service.user.get_user_by_id(id).await {
        Ok(user_data) => {
            // Convert the user data to a format the template can use
            let user = serde_json::json!({
                "id": user_data.id,
                "username": user_data.username,
                "user_type_id": user_data.user_type_id,
                "is_active": user_data.is_active,
                "last_login_at": user_data.last_login_at,
                "created_at": user_data.created_at,
                "updated_at": user_data.updated_at
            });
            context.insert("user", &user);

            // Get user types for the form
            match config
                .service
                .user_type
                .get_user_type_array(ListQueryParams::default())
                .await
            {
                Ok(user_types) => {
                    context.insert("user_types", &user_types);
                }
                Err(e) => {
                    error!("Failed to load user types: {}", e);
                    context.insert("error", "사용자 유형을 불러오는 중 오류가 발생했습니다.");
                }
            }

            match config.tera.render("user_form.html", &context) {
                Ok(s) => Html(s).into_response(),
                Err(e) => {
                    error!("Template rendering error: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Template rendering error",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to load user: {}", e);
            (StatusCode::NOT_FOUND, "User not found").into_response()
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct UserFormData {
    _username: String,
    _password: Option<String>,
    _user_type_id: i64,
    _is_active: bool,
}

#[derive(Debug, Validate, Deserialize)]
pub struct CreateUserForm {
    #[validate(length(min = 3, message = "Username must be at least 3 characters long"))]
    username: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    password: String,
    _user_type_id: i64,
    _is_active: Option<bool>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct UpdateUserForm {
    #[validate(length(min = 3, message = "Username must be at least 3 characters long"))]
    username: Option<String>,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    password: Option<String>,
    _user_type_id: Option<i64>,
    _is_active: Option<bool>,
}
