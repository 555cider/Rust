mod auth;
mod dashboard;
mod history;
mod permission;
mod user;
mod user_type;

use crate::AppState;
use axum::Router;

pub fn route() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::route())
        .nest("/dashboard", dashboard::route())
        .nest("/history", history::routes())
        .nest("/permission", permission::route())
        .nest("/user", user::route())
        .nest("/user-type", user_type::route())
}
