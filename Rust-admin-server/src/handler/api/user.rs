use crate::{
    errors::AppError,
    model::dto::{common::ListQueryParams, user::CreateUserRequest},
    AppState,
};
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/", get(get_user).post(post_user))
        .route("/{id}", get(get_user_by_id))
}

async fn post_user(
    State(config): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let response = config.service.user.create_user(req).await?;
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

async fn get_user(
    State(config): State<AppState>,
    Query(query): Query<ListQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    let response = config.service.user.get_user_array(query).await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}

async fn get_user_by_id(
    State(config): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let response = config.service.user.get_user_by_id(id).await?;
    Ok((StatusCode::OK, Json(response)).into_response())
}
