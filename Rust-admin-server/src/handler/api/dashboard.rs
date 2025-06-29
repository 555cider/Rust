use crate::{
    errors::AppError,
    filter::{auth, UserId},
    model::dto::dashboard::DashboardData,
    AppState,
};
use axum::{extract::State, middleware, response::Json, routing::get, Extension, Router};
use serde::Serialize;

#[derive(Serialize)]
struct ApiDashboardResponse {
    dashboard_data: DashboardData,
}

pub fn route() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(api_dashboard_data))
        .layer(middleware::from_fn(auth))
}

async fn api_dashboard_data(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Json<ApiDashboardResponse>, AppError> {
    // Extract the numeric ID from the UserId type
    let user_id_num = user_id.0;
    let dashboard_data = DashboardData::new(&state.pool, user_id_num).await?;
    Ok(Json(ApiDashboardResponse { dashboard_data }))
}
