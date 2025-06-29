use crate::{filter::auth, filter::UserId, AppState};
use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use tera::Context;
use tracing::error;
use validator::Validate;

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/", get(permissions_page))
        .route("/new", get(permission_create_page))
        .route("/edit/{id}", get(permission_edit_page))
}

async fn permissions_page(
    State(config): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "권한 관리");
    context.insert("active_page", "permissions");
    context.insert("user_id", &user_id.0);

    // Add current user info for the template
    if let Ok(current_user) = config.service.user.get_user_by_id(user_id.0).await {
        context.insert("current_user", &current_user);
    }

    match config.tera.render("permission.html", &context) {
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

async fn permission_create_page(
    State(config): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "권한 추가");
    context.insert("active_page", "permissions");
    context.insert("user_id", &user_id.0);

    // Add current user info for the template
    if let Ok(current_user) = config.service.user.get_user_by_id(user_id.0).await {
        context.insert("current_user", &current_user);
    }

    // Create a new permission with default values
    let permission = serde_json::json!({
        "id": null,
        "code": "",
        "name": ""
    });
    context.insert("permission", &permission);

    match config.tera.render("permission_form.html", &context) {
        Ok(s) => Html(s).into_response(),
        Err(e) => {
            error!("Template rendering error: {}\nContext: {:#?}", e, context);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template rendering error: {}", e),
            )
                .into_response()
        }
    }
}

async fn permission_edit_page(
    State(config): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(permission_id): Path<i32>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "권한 수정");
    context.insert("active_page", "permissions");
    context.insert("user_id", &user_id.0);

    // Add current user info for the template
    if let Ok(current_user) = config.service.user.get_user_by_id(user_id.0).await {
        context.insert("current_user", &current_user);
    }

    // Get permission data
    match config
        .service
        .permission
        .get_permission_by_id(permission_id)
        .await
    {
        Ok(permission) => {
            tracing::debug!("Successfully retrieved permission: {:?}", permission);
            context.insert("permission", &permission);

            match config.tera.render("permission_form.html", &context) {
                Ok(s) => Html(s).into_response(),
                Err(e) => {
                    error!("Template rendering error: {}\nContext: {:#?}", e, context);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Template rendering error: {}", e),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to load permission with ID {}: {}", permission_id, e);
            let error_message = format!(
                "권한을 찾을 수 없습니다 (ID: {})\n에러: {}",
                permission_id, e
            );

            // For API requests, return a JSON response
            if let Some(accept) = headers.get("accept") {
                if accept.to_str().unwrap_or("").contains("application/json") {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!({ "error": error_message })),
                    )
                        .into_response();
                }
            }

            // For HTML requests, show an error page
            let mut error_context = Context::new();
            error_context.insert("title", "권한을 찾을 수 없음");
            error_context.insert("error_message", &error_message);
            error_context.insert("back_url", "/permission");

            if let Ok(html) = config.tera.render("error.html", &error_context) {
                (StatusCode::NOT_FOUND, Html(html)).into_response()
            } else {
                (StatusCode::NOT_FOUND, error_message).into_response()
            }
        }
    }
}

#[derive(Debug, Validate, Deserialize)]
pub struct CreatePermissionForm {
    #[validate(length(min = 1, message = "Code is required"))]
    pub code: String,
    pub _name: Option<String>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct UpdatePermissionForm {
    pub _code: Option<String>,
    pub _name: Option<String>,
}
