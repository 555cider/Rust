use crate::{errors::AppError, AppState};
use axum::response::IntoResponse;
use serde::Serialize;
use tera::Context;
use tracing::error;

/// Helper function to render templates with error handling
pub async fn render_template<T: Serialize + std::fmt::Debug>(
    state: &AppState,
    template_name: &str,
    context: T,
) -> Result<axum::response::Response, AppError> {
    // Check if template exists
    if !state
        .tera
        .get_template_names()
        .any(|name| name == template_name)
    {
        return Err(AppError::InternalServerError(format!(
            "Template '{}' not found",
            template_name
        )));
    }

    // Convert context to Tera Context
    let tera_context = Context::from_serialize(&context).map_err(|e| {
        AppError::InternalServerError(format!("Failed to create template context: {}", e))
    })?;

    // Render the template
    state
        .tera
        .render(template_name, &tera_context)
        .map(|html| axum::response::Html(html).into_response())
        .map_err(|e| {
            error!("Failed to render template '{}': {}", template_name, e);
            AppError::InternalServerError(format!("Failed to render template '{}'", template_name))
        })
}
