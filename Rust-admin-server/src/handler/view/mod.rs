pub mod auth;
pub mod dashboard;
pub mod history;
pub mod permission;
pub mod profile;
pub mod settings;
pub mod user;
pub mod user_type;

use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tera::Context;
use tracing::error;

pub fn route() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::route())
        .nest("/dashboard", dashboard::route())
        .nest("/history", history::route())
        .nest("/permission", permission::route())
        .nest("/profile", profile::route())
        .nest("/settings", settings::route())
        .nest("/user", user::route())
        .nest("/user-types", user_type::route())
        .route("/", get(index))
        .route("/favicon.ico", get(|| async { StatusCode::NOT_FOUND }))
}

async fn index(State(config): State<AppState>) -> impl IntoResponse {
    let context = Context::new();
    match config.tera.render("index.html", &context) {
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
