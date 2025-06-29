use crate::{
    config::{env_loader, env_loader::AppConfig},
    errors::AppError,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64, // Subject (user id)
    pub username: String,
    pub role: String,
    pub exp: usize, // Expiration time (timestamp)
}

/// Token 생성
fn create_token(
    user_id: i64,
    username: &str,
    user_type_id: &str,
    duration: Duration,
    secret: &[u8],
) -> Result<String, AppError> {
    let expiration = Utc::now()
        .checked_add_signed(duration)
        .expect("valid timestamp")
        .timestamp();
    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        role: user_type_id.to_string(),
        exp: expiration as usize,
    };
    let header = Header::new(Algorithm::HS256);
    encode(&header, &claims, &EncodingKey::from_secret(secret)).map_err(AppError::JwtError)
}

/// Token 검증 및 Claims 반환
pub fn validate_token(token: &str) -> Result<Claims, AppError> {
    let config = env_loader::get_config();
    let decoding_key = DecodingKey::from_secret(config.token.secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|token_data| token_data.claims)
        .map_err(AppError::JwtError)
}

/// accessToken 생성: 1시간
pub fn generate_access_token(
    config: &AppConfig,
    user_id: i64,
    user_type_name: &str,
    username: &str,
) -> Result<String, AppError> {
    create_token(
        user_id,
        username,
        user_type_name,
        Duration::seconds(config.token.access_exp),
        config.token.secret.as_ref(),
    )
}

/// refreshToken 생성: 1주일
pub fn generate_refresh_token(
    config: &AppConfig,
    user_id: i64,
    user_type_name: &str,
    username: &str,
) -> Result<String, AppError> {
    create_token(
        user_id,
        username,
        user_type_name,
        Duration::seconds(config.token.refresh_exp),
        config.token.secret.as_ref(),
    )
}
