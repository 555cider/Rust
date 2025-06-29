use crate::{
    errors::AppError,
    filter::{auth, UserId},
    model::dto::history::HistoryListQuery,
    AppState,
};
use axum::{
    extract::{Extension, Path, Query, State},
    middleware::from_fn,
    response::Json,
    routing::{delete, get},
    Router,
};
use chrono::Utc;
use serde_json::json;
use tracing::info;
use uuid::Uuid;

// Default number of days to keep history
const DEFAULT_RETENTION_DAYS: i64 = 90;

/// Create router for history endpoints
pub fn routes() -> Router<AppState> {
    // Create a route group with the auth middleware
    let history_routes = Router::new()
        // List all history with pagination and filtering
        .route(
            "/",
            get(|state, auth, query| list_history(state, auth, query)),
        )
        // Get recent history
        .route(
            "/recent",
            get(|state, auth, query| get_recent_history(state, auth, query)),
        )
        // Get a specific history by ID
        .route(
            "/{id}",
            get(|state, auth, path| get_history(state, auth, path)),
        )
        // Clean up old history
        .route(
            "/cleanup",
            delete(|state, auth, params| cleanup_old_logs(state, auth, params)),
        )
        .layer(from_fn(auth));

    // Return the history routes
    history_routes
}

/// List history with pagination and filtering
///
/// # Parameters
/// - `page`: Page number (default: 1)
/// - `limit`: Number of items per page (default: 20, max: 100)
/// - `user_id`: Filter by user ID
/// - `action`: Filter by action name
/// - `entity_id`: Filter by entity ID
/// - `ip_address`: Filter by IP address
/// - `start_date`: Filter by start date (ISO 8601 format)
/// - `end_date`: Filter by end date (ISO 8601 format)
/// - `search`: Search term to filter by (searches in action and details)
///
/// # Permissions
/// - Admin users can view all history
/// - Regular users can only view their own history
async fn list_history(
    State(state): State<AppState>,
    Extension(auth): Extension<Option<UserId>>,
    Query(mut query): Query<HistoryListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Check if user is authenticated
    let user_id =
        auth.ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))?;
    info!("Listing history with query: {:?}", query);

    // Check if user has admin role
    let has_permission = state
        .service
        .permission
        .has_permission(user_id.0, "history:read_all")
        .await?;

    // If user doesn't have permission, they can only see their own history
    if !has_permission {
        query.user_id = Some(user_id.0);
    }

    // Get paginated logs from the service
    let logs = state
        .service
        .history
        .get_recent_history(query.clone())
        .await?;

    // Get total count for pagination
    let total = logs.len() as i64;

    // Prepare pagination metadata
    let page = query.get_page();
    let limit = query.get_limit();
    let total_pages = if total > 0 {
        ((total - 1) / limit) + 1
    } else {
        1
    };

    let response = json!({
        "success": true,
        "data": logs,
        "pagination": {
            "total": total,
            "page": page,
            "limit": limit,
            "total_pages": total_pages,
            "has_next": page < total_pages,
            "has_previous": page > 1,
        },
        "timestamp": Utc::now().to_rfc3339(),
        "request_id": Uuid::new_v4().to_string(),
    });

    Ok(Json(response))
}

/// Get recent history
///
/// Returns the most recent history (default: 10)
///
/// # Parameters
/// - `limit`: Number of items to return (default: 10, max: 100)
/// - `user_id`: Filter by user ID
/// - `action`: Filter by action name
/// - `entity_id`: Filter by entity ID
///
/// # Permissions
/// - Admin users can view all history
/// - Regular users can only view their own history
async fn get_recent_history(
    State(state): State<AppState>,
    Extension(auth): Extension<Option<UserId>>,
    Query(mut query): Query<HistoryListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Check if user is authenticated
    let user_id =
        auth.ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))?;
    info!("Getting recent history");

    // Default to 10 most recent logs if no limit is specified
    if query.limit.is_none() {
        query.limit = Some(10);
    } else {
        // Ensure limit is reasonable
        query.limit = Some(query.limit.unwrap().min(100));
    }

    // Check if user has admin role
    let has_permission = state
        .service
        .permission
        .has_permission(user_id.0, "history:read_all")
        .await?;

    // If user doesn't have permission, they can only see their own history
    if !has_permission {
        query.user_id = Some(user_id.0);
    }

    // Get paginated logs from the service
    let logs = state.service.history.get_recent_history(query).await?;

    let response = json!({
        "success": true,
        "data": logs,
        "timestamp": Utc::now().to_rfc3339(),
        "request_id": Uuid::new_v4().to_string(),
    });

    Ok(Json(response))
}

/// Get a specific history by ID
///
/// # Permissions
/// - Admin users can view any history
/// - Regular users can only view their own history
async fn get_history(
    State(state): State<AppState>,
    Extension(auth): Extension<Option<UserId>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Check if user is authenticated
    let user_id =
        auth.ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))?;
    info!("Getting history with ID: {}", id);

    // Get the history
    let log = state
        .service
        .history
        .get_history_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("History not found".to_string()))?;

    // Check if user has permission to read all history
    let has_permission = state
        .service
        .permission
        .has_permission(user_id.0, "history:read_all")
        .await?;

    // If user doesn't have permission and is not the owner of the log, deny access
    if !has_permission && log.user_id != Some(user_id.0) {
        return Err(AppError::Forbidden(
            "You don't have permission to view this history".to_string(),
        ));
    }

    let response = json!({
        "success": true,
        "data": log,
        "message": "",
        "timestamp": Utc::now().to_rfc3339(),
        "request_id": Uuid::new_v4().to_string(),
    });

    Ok(Json(response))
}

/// Clean up old history
///
/// Deletes history older than the specified number of days (default: 90 days)
///
/// # Parameters
/// - `days`: Number of days of logs to keep (default: 90, min: 1, max: 3650)
///
/// # Permissions
/// - Requires admin privileges
async fn cleanup_old_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<Option<UserId>>,
    Query(params): Query<std::collections::HashMap<String, i64>>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Check if user is authenticated
    let user_id =
        auth.ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))?;

    // Check if user has permission to delete logs
    let has_permission = state
        .service
        .permission
        .has_permission(user_id.0, "history:delete")
        .await
        .unwrap_or(false); // Default to false if there's an error

    if !has_permission {
        return Err(AppError::Forbidden(
            "You don't have permission to perform this action".to_string(),
        ));
    }

    // Get the number of days from query params or use default
    let days = params
        .get("days")
        .copied()
        .unwrap_or(DEFAULT_RETENTION_DAYS);

    // Ensure days is within a reasonable range
    if days < 1 || days > 3650 {
        // 10 years max
        return Err(AppError::BadRequest(
            "Days must be between 1 and 3650".to_string(),
        ));
    }

    info!(
        "User {} initiated cleanup of history older than {} days",
        user_id.0, days
    );

    // Delete old logs
    let deleted = state.service.history.cleanup_old_logs(days).await?;

    info!(
        "User {} completed cleanup of {} old history",
        user_id.0, deleted
    );

    let response = json!({
        "success": true,
        "data": {
            "deleted": deleted,
            "days": days,
        },
        "message": format!("Successfully deleted {} old history", deleted),
        "timestamp": Utc::now().to_rfc3339(),
        "request_id": Uuid::new_v4().to_string(),
    });

    Ok(Json(response))
}
