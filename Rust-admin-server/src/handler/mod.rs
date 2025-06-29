mod api;
mod view;

use crate::AppState;
use axum::Router;
use tower_http::services::ServeDir;

pub fn route() -> Router<AppState> {
    Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .nest("/api", api::route())
        .merge(view::route())
}
