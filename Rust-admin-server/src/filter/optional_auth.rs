use crate::{
    filter::auth::UserId,
    util::{cookie_util, header_util, token_util},
};
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::convert::Infallible;
use tracing::warn;

/// Middleware that optionally authenticates requests using a JWT token from the Authorization header or access_token cookie.
/// If the token is valid, the user ID is added to the request extensions.
/// If the token is missing or invalid, the request continues without authentication.
pub async fn optional_auth(mut request: Request<Body>, next: Next) -> Result<Response, Infallible> {
    // Clone the headers to avoid moving the request
    let headers = request.headers().clone();

    // 1. Try to extract token from Authorization header
    let token = header_util::extract_token_from_header(&headers)
        // 2. If not found, try to get from access_token cookie
        .map(|s| s.to_string())
        .or_else(|| cookie_util::get_cookie_value(&headers, "access_token"));

    // 3. If token exists, validate it
    if let Some(token_str) = token {
        match token_util::validate_token(&token_str) {
            Ok(claims) => {
                // Insert the user ID into request extensions
                request.extensions_mut().insert(UserId::new(claims.sub));
            }
            Err(e) => {
                warn!(error = %e, "Token validation failed");
                // Continue without authentication
            }
        }
    }

    // 4. Continue to the next middleware/handler without authentication
    Ok(next.run(request).await)
}
