use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

pub async fn setup_db_pool(database_url: &str) -> SqlitePool {
    info!("Connecting to database at `{}`", database_url);

    SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .expect("Failed to connect to the database")
}
