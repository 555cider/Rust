use axum::http::{header, HeaderMap};

/// Extracts the token from the Authorization header, removing the 'Bearer ' prefix if present.
/// Returns None if the header is missing or malformed.
pub fn extract_token_from_header(headers: &HeaderMap) -> Option<&str> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(header) => header.strip_prefix("Bearer "),
        None => None,
    }
}

/// Checks if the request is an API request by examining the Accept header.
/// Returns true if the Accept header contains 'application/json'.
pub fn is_api_request(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map_or(false, |accept| accept.contains("application/json"))
}
