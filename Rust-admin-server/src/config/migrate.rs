use anyhow::anyhow;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path;
use tracing::{error, info};

pub async fn run_migrations(database_url: &str) -> anyhow::Result<()> {
    let db_path = database_url.trim_start_matches("sqlite:");
    info!("Database path: {}", db_path);

    // Check if the database exists, create it if it doesn't
    if !Path::new(db_path).exists() {
        info!("Database does not exist, creating a new one");
        if let Some(parent) = Path::new(db_path).parent() {
            if !parent.exists() {
                info!("Creating parent directory: {:?}", parent);
                std::fs::create_dir_all(parent).map_err(|e| {
                    error!("Failed to create directory: {:?}", e);
                    anyhow!("Failed to create directory: {}", e)
                })?;
            }
        }
        // Create an empty file to initialize the database
        std::fs::File::create(db_path).map_err(|e| {
            error!("Failed to create database file: {}", e);
            anyhow!("Failed to create database file: {}", e)
        })?;
        info!("Created new database file at: {}", db_path);
    }

    // Create a connection pool for migrations
    info!("Connecting to database for migrations...");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|e| {
            error!("Failed to connect to database: {}", e);
            anyhow!("Failed to connect to database: {}", e)
        })?;
    info!("Successfully connected to database");

    // Run migrations
    info!("Running database migrations...");
    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(_) => {
            info!("Database migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Migration error: {}", e);
            error!("Error details: {:?}", e);
            Err(anyhow!("Failed to run database migrations: {}", e))
        }
    }
}
