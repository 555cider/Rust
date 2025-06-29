use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Form, Router,
};
use serde::Deserialize;
use std::collections::HashMap;
use tera::Context;
use tracing::{debug, error};

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page).post(login_handler))
        .route("/register", get(register_page))
}

async fn login_page(
    State(config): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "로그인");
    context.insert("active_page", "login");

    if let Some(error_message) = query.get("error") {
        context.insert("error", error_message);
    }

    match config.tera.render("login.html", &context) {
        Ok(s) => Html(s).into_response(),
        Err(e) => {
            error!("Template rendering error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template rendering error: {}", e),
            )
                .into_response()
        }
    }
}

async fn login_handler(
    State(_config): State<AppState>,
    Form(_form): Form<LoginForm>,
) -> impl IntoResponse {
    // TODO: Implement actual authentication
    debug!("Login handler called");

    // On successful login, redirect to dashboard
    Redirect::to("/dashboard").into_response()
}

async fn register_page(State(config): State<AppState>) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("title", "회원가입");
    context.insert("active_page", "register");

    match config.tera.render("register.html", &context) {
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

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    _username: String,
    _password: String,
    _remember_me: Option<String>,
}
