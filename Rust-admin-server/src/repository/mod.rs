pub mod auth;
pub mod history;
pub mod permission;
pub mod user;
pub mod user_type;

use async_trait::async_trait;
pub use auth::AuthRepository;
pub use history::HistoryRepository;
pub use permission::PermissionRepository;
use sqlx::SqlitePool;
use std::sync::Arc;
pub use user::UserRepository;
pub use user_type::UserTypeRepository;

#[async_trait]
pub trait Repository: Send + Sync + 'static {
    type Pool;
    #[allow(dead_code)]
    fn new(pool: Arc<SqlitePool>) -> Self;
}

// Implement Repository for all repository types
macro_rules! impl_repository {
    ($repo:ty) => {
        #[async_trait::async_trait]
        impl Repository for $repo {
            type Pool = SqlitePool;

            fn new(pool: Arc<SqlitePool>) -> Self {
                Self::new(pool)
            }
        }
    };
}

// Implement Repository for all repository types
impl_repository!(AuthRepository);
impl_repository!(HistoryRepository);
impl_repository!(PermissionRepository);
impl_repository!(UserRepository);
impl_repository!(UserTypeRepository);
