use crate::{errors::AppError, filter::auth, filter::UserId, AppState};
use axum::{
    extract::{Extension, State},
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
};
use tera::Context;
use tracing::{debug, error, info};

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/", get(settings_page))
}

use crate::model::dto::user::UserResponse;

#[derive(serde::Serialize, Debug)]
struct TemplateContext {
    title: &'static str,
    active_page: &'static str,
    user_id: i64,
    current_user: Option<UserResponse>,
}

impl From<TemplateContext> for Context {
    fn from(ctx: TemplateContext) -> Self {
        let mut context = Context::new();
        context.insert("title", &ctx.title);
        context.insert("active_page", &ctx.active_page);
        context.insert("user_id", &ctx.user_id);
        if let Some(user) = &ctx.current_user {
            context.insert("current_user", user);
        }
        context
    }
}

async fn settings_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<impl IntoResponse, AppError> {
    info!("Starting settings page handler for user_id: {}", user_id.0);

    // Get current user data with detailed error handling
    let current_user = match state.service.user.get_user_by_id(user_id.0).await {
        Ok(user) => {
            debug!("Found user: {:?}", user);
            Some(user)
        }
        Err(e) => {
            error!("Error fetching user: {}", e);
            None
        }
    };

    // Create Tera context directly
    let mut tera_context = Context::new();

    // Add basic fields
    tera_context.insert("title", "설정");
    tera_context.insert("active_page", "settings");
    tera_context.insert("user_id", &user_id.0);

    // Add current user if available
    if let Some(user) = &current_user {
        let user_value = serde_json::to_value(user).map_err(|e| {
            error!("Failed to serialize user: {}", e);
            AppError::InternalServerError("Failed to serialize user data".to_string())
        })?;

        tera_context.insert("current_user", &user_value);
    } else {
        // Insert an empty object if no user is available
        tera_context.insert("current_user", &serde_json::json!({}));
    }

    // Debug: Log context information
    debug!(
        "Template context created with title: {}",
        tera_context.get("title").unwrap_or(&tera::Value::Null)
    );

    // Get template names for debugging
    let template_names: Vec<&str> = state.tera.get_template_names().collect();
    debug!("Available templates: {:?}", template_names);

    // Check if template exists
    if !template_names.iter().any(|&name| name == "settings.html") {
        error!("Template 'settings.html' not found in the loaded templates");
        return Err(AppError::InternalServerError(
            "Template not found".to_string(),
        ));
    }

    // Try to render the template
    match state.tera.render("settings.html", &tera_context) {
        Ok(html) => {
            debug!("Successfully rendered template");
            Ok(axum::response::Html(html).into_response())
        }
        Err(e) => {
            // Log detailed error information
            error!("Template rendering error: {}", e);

            // Log the error source if available
            use std::error::Error;
            if let Some(source) = e.source() {
                error!("Source error: {}", source);
            }

            // Log the error as a string for debugging
            let error_string = e.to_string();
            error!("Error details: {}", error_string);

            // Create a simple error response
            Err(AppError::InternalServerError(format!(
                "Template rendering error: {}",
                error_string
            )))
        }
    }
}
