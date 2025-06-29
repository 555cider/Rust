use crate::{
    errors::AppError,
    util::{cookie_util, header_util, token_util},
};
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use tracing::warn;

/// Wrapper type for user ID that implements Send + Sync + 'static
#[derive(Debug, Clone)]
pub struct UserId(pub i64);

impl UserId {
    /// Creates a new UserId with the given user ID.
    pub fn new(user_id: i64) -> Self {
        Self(user_id)
    }
}

/// Middleware that authenticates requests using a JWT token from the Authorization header or access_token cookie.
/// Returns an error if the token is missing, invalid, or expired.
/// Renders an error page for unauthorized access
async fn render_unauthorized_page() -> Response {
    // Create a simple error page
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Unauthorized</title>
        <script src="https://cdn.tailwindcss.com"></script>
    </head>
    <body class="bg-gray-100 flex items-center justify-center min-h-screen">
        <div class="bg-white p-8 rounded-lg shadow-md max-w-md w-full">
            <div class="text-center">
                <h1 class="text-2xl font-bold text-red-600 mb-4">Unauthorized Access</h1>
                <p class="text-gray-700 mb-6">You need to be logged in to access this page.</p>
                <a href="/auth/login" class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 transition-colors">
                    Go to Login
                </a>
            </div>
        </div>
    </body>
    </html>
    "#;

    (StatusCode::UNAUTHORIZED, Html(html)).into_response()
}

/// Middleware that authenticates requests using a JWT token from the Authorization header or access_token cookie.
/// Returns an error if the token is missing, invalid, or expired.
pub async fn auth(headers: HeaderMap, mut request: Request, next: Next) -> Response {
    // Check if this is an API request
    let is_api_request = header_util::is_api_request(&headers);

    // 1. Extract token from Authorization header
    let token = header_util::extract_token_from_header(&headers)
        // 2. If not found in header, try to get from access_token cookie
        .map(|s| s.to_string())
        .or_else(|| cookie_util::get_cookie_value(&headers, "access_token"));

    let token = match token {
        Some(token) if !token.is_empty() => token,
        _ => {
            warn!("Missing or invalid Authorization header and access_token cookie");
            return if is_api_request {
                AppError::Unauthorized(
                    "Missing or invalid Authorization header or access_token cookie. Please login again."
                        .to_string(),
                )
                    .into_response()
            } else {
                render_unauthorized_page().await
            };
        }
    };

    // Validate the token
    match token_util::validate_token(&token) {
        Ok(claims) => {
            // Add the user ID to the request extensions for use in handlers
            request.extensions_mut().insert(UserId::new(claims.sub));
            next.run(request).await
        }
        Err(e) => {
            warn!(error = %e, "Token validation failed");
            if is_api_request {
                AppError::Unauthorized("Invalid or expired token".to_string()).into_response()
            } else {
                render_unauthorized_page().await
            }
        }
    }
}
