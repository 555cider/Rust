use crate::{
    errors::AppError,
    filter::{auth, UserId},
    model::dto::{
        common::ListQueryParams,
        user::UserResponse,
        user_type::{CreateUserTypeRequest, UpdateUserTypeRequest, UserTypeResponse},
    },
    util::template_util,
    AppState,
};
use axum::{
    extract::{Form, Path, Query, State},
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get},
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use tera::Context;

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/", get(user_type_list).post(create_user_type))
        .route("/create", get(user_type_create_page))
        .route(
            "/edit/{id}",
            get(user_type_edit_page).post(update_user_type),
        )
        .route("/{id}", delete(delete_user_type))
}

#[derive(Debug, Deserialize)]
pub struct UserTypeListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub q: Option<String>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TemplateContext {
    title: &'static str,
    active_page: &'static str,
    user_id: i64,
    current_user: Option<UserResponse>,
    user_types: Option<Vec<UserTypeResponse>>,
    user_type: Option<UserTypeResponse>,
    total: i64,
    page: i64,
    per_page: i64,
    total_pages: i64,
    q: Option<String>,
    sort_by: Option<String>,
    order: Option<String>,
    status: Option<String>,
}

impl From<TemplateContext> for Context {
    fn from(ctx: TemplateContext) -> Self {
        let mut context = Context::new();
        context.insert("title", &ctx.title);
        context.insert("active_page", &ctx.active_page);
        context.insert("user_id", &ctx.user_id);
        context.insert("current_user", &ctx.current_user);
        context.insert("user_types", &ctx.user_types);
        context.insert("user_type", &ctx.user_type);
        context.insert("total", &ctx.total);
        context.insert("page", &ctx.page);
        context.insert("per_page", &ctx.per_page);
        context.insert("total_pages", &ctx.total_pages);
        context.insert("q", &ctx.q);
        context.insert("sort_by", &ctx.sort_by);
        context.insert("order", &ctx.order);
        context.insert("status", &ctx.status);
        context
    }
}

// Using template_util::render_template instead of local implementation

pub async fn user_type_list(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Query(query): Query<UserTypeListQuery>,
) -> Result<Response, AppError> {
    // Get current user info for the template
    let current_user = state.service.user.get_user_by_id(user_id.0).await.ok();

    // Set up pagination
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let _offset = (page - 1) * limit;

    // Prepare query parameters
    let query_params = ListQueryParams {
        page: Some(page),
        limit: Some(limit),
        sort_by: query.sort_by.clone(),
        order: query.order.clone(),
        q: query.q.clone(),
        status: query.status.clone(),
    };

    let user_types = state
        .service
        .user_type
        .get_user_type_array(query_params)
        .await?;

    // For now, we'll set total to the length of user_types
    // In a real app, you might want to get the total count separately
    let total = user_types.len() as i64;

    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    let context = TemplateContext {
        title: "사용자 유형 관리",
        active_page: "user_types",
        user_id: user_id.0,
        current_user,
        user_types: Some(user_types),
        user_type: None,
        total,
        page,
        per_page: limit,
        total_pages,
        q: query.q,
        sort_by: query.sort_by,
        order: query.order,
        status: query.status,
    };

    template_util::render_template(&state, "user_type.html", context).await
}

pub async fn user_type_create_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Response, AppError> {
    let current_user = state.service.user.get_user_by_id(user_id.0).await.ok();

    let context = TemplateContext {
        title: "사용자 유형 생성",
        active_page: "user_types",
        user_id: user_id.0,
        current_user,
        user_types: None,
        user_type: None,
        total: 0,
        page: 1,
        per_page: 10,
        total_pages: 1,
        q: None,
        sort_by: None,
        order: None,
        status: None,
    };

    template_util::render_template(&state, "user_type_form.html", context).await
}

pub async fn user_type_edit_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(user_type_id): Path<i64>,
) -> Result<Response, AppError> {
    let current_user = state.service.user.get_user_by_id(user_id.0).await.ok();
    let user_type = state
        .service
        .user_type
        .get_user_type_by_id(user_type_id)
        .await?;

    let context = TemplateContext {
        title: "사용자 유형 수정",
        active_page: "user_types",
        user_id: user_id.0,
        current_user,
        user_types: None,
        user_type: Some(user_type),
        total: 0,
        page: 1,
        per_page: 10,
        total_pages: 1,
        q: None,
        sort_by: None,
        order: None,
        status: None,
    };

    template_util::render_template(&state, "user_type_form.html", context).await
}

pub async fn create_user_type(
    State(state): State<AppState>,
    Extension(_user_id): Extension<UserId>,
    Form(payload): Form<CreateUserTypeRequest>,
) -> Result<Response, AppError> {
    state.service.user_type.create_user_type(payload).await?;
    Ok(Redirect::to("/user-types").into_response())
}

pub async fn update_user_type(
    State(state): State<AppState>,
    Extension(_user_id): Extension<UserId>,
    Path(user_type_id): Path<i64>,
    Form(payload): Form<UpdateUserTypeRequest>,
) -> Result<Response, AppError> {
    state
        .service
        .user_type
        .update_user_type(user_type_id, payload)
        .await?;
    Ok(Redirect::to("/user-types").into_response())
}

pub async fn delete_user_type(
    State(state): State<AppState>,
    Extension(_user_id): Extension<UserId>,
    Path(user_type_id): Path<i64>,
) -> Result<Response, AppError> {
    state
        .service
        .user_type
        .delete_user_type(user_type_id)
        .await?;
    Ok(Redirect::to("/user-types").into_response())
}
