use crate::config::env_loader::AppConfig;
use axum::http::{header, HeaderMap};
use cookie::{Cookie, CookieBuilder, SameSite};
use time::Duration;

pub fn create_access_token_cookie(config: &AppConfig, value: &str) -> Cookie<'static> {
    create_cookie(
        config.cookie.access_token_name.to_string(),
        value.to_string(),
        config.cookie.access_token_max_age,
        config.cookie.secure,
    )
}

pub fn create_refresh_token_cookie(config: &AppConfig, value: &str) -> Cookie<'static> {
    create_cookie(
        config.cookie.refresh_token_name.to_string(),
        value.to_string(),
        config.cookie.refresh_token_max_age,
        config.cookie.secure,
    )
}

pub fn get_refresh_token_cookie_value(config: &AppConfig, headers: &HeaderMap) -> Option<String> {
    get_cookie_value(headers, config.cookie.refresh_token_name.as_str())
}

fn create_cookie(name: String, value: String, max_age: i64, is_secure: bool) -> Cookie<'static> {
    CookieBuilder::new(name, value)
        .http_only(true)
        .secure(is_secure)
        .same_site(SameSite::Lax)
        .path("/") // Path 옵션 명시적으로 추가
        .max_age(Duration::seconds(max_age))
        .build()
}

#[allow(dead_code)]
fn _delete_cookie(name: &str, secure_cookies: bool) -> Cookie {
    CookieBuilder::new(name, "")
        .http_only(true)
        .secure(secure_cookies)
        .same_site(SameSite::Lax)
        .max_age(Duration::seconds(0))
        .build()
}

/// Gets a cookie value by name from the request headers.
/// Returns None if the cookie is not found.
pub fn get_cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|cookie_header| cookie_header.to_str().ok())
        .and_then(|cookies_str| {
            cookies_str.split(';').find_map(|cookie| {
                let cookie = cookie.trim();
                if let Some((k, v)) = cookie.split_once('=') {
                    if k == name {
                        Some(v.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        })
}
