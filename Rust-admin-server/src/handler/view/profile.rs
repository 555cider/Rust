use crate::{
    errors::AppError, filter::auth, filter::UserId, model::dto::user::UserResponse, AppState,
};
use axum::{
    extract::{Extension, State},
    middleware,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::Serialize;
use tera::Context;
use tracing::error;

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/", get(profile_page))
}

#[derive(Debug, Serialize)]
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

async fn profile_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<impl IntoResponse, AppError> {
    // Get current user data
    let current_user = state.service.user.get_user_by_id(user_id.0).await.ok();

    let context = TemplateContext {
        title: "프로필",
        active_page: "profile",
        user_id: user_id.0,
        current_user,
    };

    match state.tera.render("profile.html", &Context::from(context)) {
        Ok(html) => Ok(Html(html).into_response()),
        Err(e) => {
            error!("Template rendering error: {}", e);
            Err(AppError::InternalServerError(
                "Failed to render profile page".to_string(),
            ))
        }
    }
}
