use crate::{
    config::{auth::authn_user::AuthnUser, env_loader::AppConfig},
    errors::AppError,
    model::{dto::auth::CurrentUserResponse, dto::auth::LoginRequest, dto::auth::RegisterRequest},
    repository::{auth::AuthRepository, user::UserRepository, user_type::UserTypeRepository},
    service::history::HistoryService,
    util::{password_util, token_util},
};
use std::sync::Arc;
use tracing::{error, info, warn};
use validator::Validate;

pub struct AuthService {
    auth_repo: AuthRepository,
    user_repo: UserRepository,
    user_type_repo: UserTypeRepository,
    history: Arc<HistoryService>,
}

impl AuthService {
    pub fn new(
        auth_repo: AuthRepository,
        user_repo: UserRepository,
        user_type_repo: UserTypeRepository,
        history: Arc<HistoryService>,
    ) -> Self {
        Self {
            auth_repo,
            user_repo,
            user_type_repo,
            history,
        }
    }

    pub async fn login(
        &self,
        config: &AppConfig,
        req: LoginRequest,
        ip_address: Option<String>,
        _user_agent: Option<String>,
    ) -> Result<(String, String), AppError> {
        req.validate()?;
        info!("Login attempt for username: {}", req.username);

        let user = match self.auth_repo.find_user_by_username(&req.username).await? {
            Some(user) => user,
            None => {
                // Log failed login attempt (user not found)
                if let Err(e) = self
                    .history
                    .log_login_failed(
                        &req.username,
                        "user_not_found".to_string(),
                        ip_address.clone(),
                    )
                    .await
                {
                    error!("Failed to log failed login attempt: {}", e);
                }

                return Err(AppError::Unauthorized(
                    "Invalid username or password".to_string(),
                ));
            }
        };

        if !password_util::verify_password(&req.password, &user.password).await? {
            // Log failed login attempt (wrong password)
            if let Err(e) = self
                .history
                .log_login_failed(
                    &req.username,
                    "invalid_password".to_string(),
                    ip_address.clone(),
                )
                .await
            {
                error!("Failed to log failed login attempt: {}", e);
            }

            return Err(AppError::Unauthorized(
                "Invalid username or password".to_string(),
            ));
        }

        // Clone username before moving it
        let username = user.username.clone();

        // Log successful login
        if let Err(e) = self
            .history
            .log_login_success(user.id, Some(username.clone()), ip_address.clone())
            .await
        {
            error!("Failed to log successful login: {}", e);
        }

        info!("User {} logged in successfully", user.id);
        let access_token =
            token_util::generate_access_token(config, user.id, &user.role, &username)?;

        let refresh_token =
            token_util::generate_refresh_token(config, user.id, &user.role, &username)?;

        // Save refresh token to database
        self.auth_repo
            .save_refresh_token(user.id, &refresh_token)
            .await?;

        // Update last login time
        self.user_repo.update_last_login(user.id).await?;

        Ok((access_token, refresh_token))
    }

    pub async fn register(
        &self,
        req: RegisterRequest,
        ip_address: Option<String>,
        _user_agent: Option<String>,
    ) -> Result<i64, AppError> {
        req.validate()?;
        info!("Register request for username: {}", req.username);

        // Check if username already exists
        if self
            .auth_repo
            .find_user_by_username(&req.username)
            .await?
            .is_some()
        {
            return Err(AppError::BadRequest("Username already exists".to_string()));
        }

        // Check if user type exists and get its name
        let _user_type_info = self
            .user_type_repo
            .find_by_id(req.user_type_id)
            .await
            .map_err(|_| AppError::BadRequest("Invalid user type".to_string()))?;

        let hashed_password = password_util::hash_password(&req.password).await?;

        // Clone username to avoid moving it
        let username = req.username.clone();

        let user_id = self
            .auth_repo
            .create_user(
                username,
                hashed_password,
                req.user_type_id,
                true, // is_active
            )
            .await?;

        // Log the registration
        self.history
            .create_log(
                Some(user_id),
                "user_created",
                Some(user_id),
                Some(serde_json::json!({ "username": &req.username })),
                ip_address,
                None, // user_agent
            )
            .await?;

        info!("User registered successfully: {}", user_id);
        Ok(user_id)
    }

    pub async fn refresh_access_token(
        &self,
        config: &AppConfig,
        refresh_token: String,
        ip_address: Option<String>,
        _user_agent: Option<String>,
    ) -> Result<(String, String), AppError> {
        let claims = match token_util::validate_token(&refresh_token) {
            Ok(claims) => claims,
            Err(e) => {
                warn!("Invalid refresh token: {}", e);
                return Err(AppError::Unauthorized("Invalid refresh token".to_string()));
            }
        };

        // Verify the token is not expired
        if claims.exp < chrono::Utc::now().timestamp() as usize {
            warn!("Expired refresh token used for user: {}", claims.sub);
            return Err(AppError::Unauthorized("Refresh token expired".to_string()));
        }

        // Get the user from the database
        let user = self.user_repo.find_by_id(claims.sub).await.map_err(|e| {
            warn!("User not found for refresh token: {} - {}", claims.sub, e);
            AppError::Unauthorized("User not found".to_string())
        })?;

        // Get user type info
        let user_type_info = self
            .user_type_repo
            .get_user_type_info(user.user_type_id)
            .await?;

        // Get user type name or default to "user" if not found
        let user_type_name = user_type_info
            .as_ref()
            .map(|ut| ut.name.as_str())
            .unwrap_or("user");

        // Clone username to avoid moving it
        let username = user.username.clone();

        // Generate new tokens
        let access_token =
            token_util::generate_access_token(&config, user.id, user_type_name, &username)?;

        let refresh_token =
            token_util::generate_refresh_token(&config, user.id, user_type_name, &username)?;

        // Log token refresh
        if let Err(e) = self
            .history
            .create_log(
                Some(user.id),
                "token_refresh",
                Some(user.id),
                None,
                ip_address,
                None, // user_agent
            )
            .await
        {
            error!("Failed to log token refresh: {}", e);
        }

        info!("Refreshed tokens for user: {}", user.id);
        Ok((access_token, refresh_token))
    }

    pub async fn get_current_user(
        &self,
        current_user: AuthnUser,
    ) -> Result<CurrentUserResponse, AppError> {
        let user_type_info = self
            .user_type_repo
            .get_user_type_info(current_user.user_type_id)
            .await?;

        Ok(CurrentUserResponse {
            id: current_user.id,
            username: current_user.username,
            user_type_id: current_user.user_type_id,
            user_type: user_type_info,
            permissions: current_user.permissions.iter().cloned().collect(),
        })
    }
}
