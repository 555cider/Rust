use crate::{
    errors::AppError,
    filter::{auth, UserId},
    model::dto::{dashboard::DashboardData, user::UserResponse},
    AppState,
};
use axum::{
    extract::{Extension, Query, State},
    middleware,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use tera::Context;

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/", get(dashboard_page))
}

#[derive(Serialize)]
struct TemplateContext {
    title: &'static str,
    active_page: &'static str,
    user_id: i64,
    current_user: Option<UserResponse>,
    dashboard_data: DashboardData,
}

impl From<TemplateContext> for Context {
    fn from(ctx: TemplateContext) -> Self {
        let mut context = Context::new();
        context.insert("title", &ctx.title);
        context.insert("active_page", &ctx.active_page);
        context.insert("user_id", &ctx.user_id);
        context.insert("dashboard_data", &ctx.dashboard_data);
        if let Some(user) = &ctx.current_user {
            context.insert("current_user", user);
        }
        context
    }
}

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    range: Option<String>,
}

pub async fn dashboard_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Query(query): Query<DashboardQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Get current user data
    let current_user = state.service.user.get_user_by_id(user_id.0).await.ok();

    // Get dashboard data with optional time range
    let range = query.range.as_deref().unwrap_or("day");
    let dashboard_data = DashboardData::with_range(&state.pool, user_id.0, range).await?;

    // Prepare template context
    let context = TemplateContext {
        title: "대시보드",
        active_page: "dashboard",
        user_id: user_id.0,
        current_user,
        dashboard_data,
    };

    // Render the template
    let rendered = state
        .tera
        .render("dashboard.html", &context.into())
        .map_err(|e| {
            tracing::error!("Template rendering error: {}", e);
            AppError::InternalServerError("Template rendering error".to_string())
        })?;

    Ok(Html(rendered))
}
