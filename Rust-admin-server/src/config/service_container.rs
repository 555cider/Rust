use crate::{
    repository::{
        history::HistoryRepository, AuthRepository, PermissionRepository, UserRepository,
        UserTypeRepository,
    },
    service::{
        auth::AuthService, history::HistoryService, permission::PermissionService,
        user::UserService, user_type::UserTypeService,
    },
};
use std::sync::Arc;

#[derive(Clone)]
pub struct ServiceContainer {
    pub auth: Arc<AuthService>,
    pub history: Arc<HistoryService>,
    pub permission: Arc<PermissionService>,
    pub user: Arc<UserService>,
    pub user_type: Arc<UserTypeService>,
}

impl ServiceContainer {
    pub fn new(db: Arc<sqlx::SqlitePool>) -> Self {
        let auth_repo = AuthRepository::new(db.clone());
        let history_repo = HistoryRepository::new(db.clone());
        let permission_repo = PermissionRepository::new(db.clone());
        let user_repo = UserRepository::new(db.clone());
        let user_type_repo = UserTypeRepository::new(db.clone());

        let history = Arc::new(HistoryService::new(history_repo));
        let auth = Arc::new(AuthService::new(
            auth_repo,
            user_repo.clone(),
            user_type_repo.clone(),
            history.clone(),
        ));

        let permission = Arc::new(PermissionService::new(permission_repo.clone()));
        let user = Arc::new(UserService::new(user_repo.clone()));
        let user_type = Arc::new(UserTypeService::new(user_type_repo.clone()));

        Self {
            auth,
            history,
            permission,
            user,
            user_type,
        }
    }
}
