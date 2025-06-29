use crate::{
    errors::AppError,
    filter::{auth, UserId},
    model::dto::{
        history::{HistoryListQuery, HistoryResponse},
        user::UserResponse,
    },
    util::render_template,
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    middleware,
    response::IntoResponse,
    routing::{get, Router},
    Extension,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

pub fn route() -> Router<AppState> {
    Router::new()
        .layer(middleware::from_fn(auth))
        .route("/recent", get(recent_history_page))
        .route("/{id}", get(history_detail_page))
        .route("/", get(history_page))
}

#[derive(Debug, Deserialize)]
pub struct HistoryQueryParams {
    page: Option<i64>,
    per_page: Option<i64>,
    user_id: Option<i64>,
    action: Option<String>,
    pub _entity_type: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TemplateContext {
    title: &'static str,
    active_page: &'static str,
    user_id: i64,
    current_user: Option<UserResponse>,
    history: Option<Vec<HistoryResponse>>,
    total: i64,
    page: i64,
    per_page: i64,
    total_pages: i64,
    id: Option<i64>,
    // Pre-calculated pagination values
    start_item: i64,
    end_item: i64,
}

async fn history_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Query(params): Query<HistoryQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    info!("Starting history page handler for user_id: {}", user_id.0);
    debug!("Query params: {:?}", params);

    // Get current user data
    let _current_user = match state.service.user.get_user_by_id(user_id.0).await {
        Ok(user) => {
            debug!("Found user: {:?}", user);
            Some(user)
        }
        Err(e) => {
            error!("Error fetching user: {}", e);
            None
        }
    };

    let page = params.page.unwrap_or(1).max(1); // Ensure page is at least 1
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100); // Ensure per_page is between 1 and 100
    let offset = (page - 1) * per_page;

    // Convert string dates to DateTime<Utc>
    let start_date = params
        .start_date
        .and_then(|d| {
            let dt = DateTime::parse_from_rfc3339(&d);
            if let Err(e) = &dt {
                error!("Error parsing start_date '{}': {}", d, e);
            }
            dt.ok()
        })
        .map(|dt| dt.with_timezone(&Utc));

    let end_date = params
        .end_date
        .and_then(|d| {
            let dt = DateTime::parse_from_rfc3339(&d);
            if let Err(e) = &dt {
                error!("Error parsing end_date '{}': {}", d, e);
            }
            dt.ok()
        })
        .map(|dt| dt.with_timezone(&Utc));

    let query = HistoryListQuery {
        limit: Some(per_page),
        offset: Some(offset),
        page: Some(page),
        per_page: Some(per_page),
        user_id: params.user_id,
        action: params.action,
        entity_id: None,
        entity_type: None,
        start_date,
        end_date: end_date.map(|dt| dt + chrono::Duration::days(1)), // Include the entire end date
    };

    debug!("Fetching history with query: {:?}", query);

    // Get history with pagination
    let history = match state
        .service
        .history
        .get_recent_history(query.clone())
        .await
    {
        Ok(history) => history,
        Err(e) => {
            error!("Error fetching history: {}", e);
            return Err(AppError::InternalServerError(
                "Failed to fetch history".to_string(),
            ));
        }
    };

    // Create a new query for counting without pagination
    let mut count_query = query.clone();
    count_query.limit = None;
    count_query.offset = None;

    // Use the service's method to get the total count
    let total = match state.service.history.count_history(&count_query).await {
        Ok(total) => total,
        Err(e) => {
            error!("Error counting history: {}", e);
            return Err(AppError::InternalServerError(
                "Failed to count history".to_string(),
            ));
        }
    };

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    // Calculate start and end items for display
    let _start_item = ((page - 1) * per_page) + 1;
    let _end_item = std::cmp::min(page * per_page, total);

    debug!(
        "Fetched {} history items (total: {}, page: {}, per_page: {})",
        history.len(),
        total,
        page,
        per_page
    );

    // Create a vector of history items with proper defaults and KST timestamps
    let history_items: Vec<HistoryResponse> = history
        .into_iter()
        .map(|mut item| {
            // Ensure required fields have proper defaults
            if item.action.is_empty() {
                item.action = "unknown".to_string();
            }
            if item.entity_type.is_none() {
                item.entity_type = Some("unknown".to_string());
            }
            if item.entity_id.is_none() {
                item.entity_id = Some(0);
            }
            if item.details.is_none() {
                item.details = Some(serde_json::json!({}));
            }
            // Convert created_at to KST (UTC+9)
            item.created_at = item.created_at + chrono::Duration::hours(9);
            item
        })
        .collect();

    // Ensure we have valid values for all template variables
    let page = page.max(1);
    let per_page = per_page.max(1);
    let total = total.max(0);
    let total_pages = total_pages.max(1);

    // Calculate pagination display values
    let start_item = ((page - 1) * per_page + 1).min(total.max(1));
    let end_item = (page * per_page).min(total);

    // Ensure we have at least an empty history items vector
    let history_items = if history_items.is_empty() {
        vec![]
    } else {
        history_items
    };

    // Get current user information
    let current_user = match state.service.user.get_user_by_id(user_id.0).await {
        Ok(user) => Some(user),
        Err(e) => {
            error!("Failed to get current user: {}", e);
            return Err(AppError::InternalServerError(
                "Failed to get current user".to_string(),
            ));
        }
    };

    // Create template context
    let template_context = TemplateContext {
        title: "활동 로그",
        active_page: "history",
        user_id: user_id.0,
        current_user,
        history: Some(history_items),
        total,
        page,
        per_page,
        total_pages: total_pages.max(1),
        id: None,
        start_item,
        end_item,
    };

    // Debug: Log available templates and context
    let template_names: Vec<&str> = state.tera.get_template_names().collect();
    debug!("Available templates: {:?}", template_names);
    debug!(
        "Total items: {}, Page: {}, Per page: {}",
        total, page, per_page
    );

    // Use the render_template helper which handles the response conversion
    render_template(&state, "history.html", template_context).await
}

/// Recent history page
async fn recent_history_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<impl IntoResponse, AppError> {
    // Get current user data
    let current_user = state.service.user.get_user_by_id(user_id.0).await.ok();
    // Create a simple query to get recent history
    let query = HistoryListQuery {
        page: Some(1),
        per_page: Some(10),
        limit: None,
        user_id: None,
        action: None,
        entity_type: None,
        entity_id: None,
        start_date: None,
        end_date: None,
        offset: None,
    };

    let history = state.service.history.get_recent_history(query).await?;
    let total = history.len() as i64;

    let context = TemplateContext {
        title: "최근 활동",
        active_page: "history",
        user_id: user_id.0,
        current_user,
        history: Some(history),
        total,
        page: 1,
        per_page: 10,
        total_pages: 1,
        id: None,
        start_item: 1,
        end_item: std::cmp::min(10, total),
    };

    render_template(&state, "recent_history.html", context).await
}

/// History detail page
async fn history_detail_page(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    // Get current user data
    let current_user = state.service.user.get_user_by_id(user_id.0).await.ok();
    // Get the history by ID
    let _history = match state.service.history.get_history_by_id(id).await? {
        Some(history) => history,
        None => return Err(AppError::NotFound("History not found".to_string())),
    };

    let context = TemplateContext {
        title: "활동 상세",
        active_page: "history",
        user_id: user_id.0,
        current_user,
        history: None,
        total: 1,
        page: 1,
        start_item: 1,
        end_item: 1,
        per_page: 1,
        total_pages: 1,
        id: Some(id),
    };

    render_template(&state, "history_detail.html", context).await
}
