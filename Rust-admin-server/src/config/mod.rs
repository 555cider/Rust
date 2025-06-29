//! Application configuration and core services.
//!
//! This module contains configuration loading, database setup, logging configuration,
//! and other core services used throughout the application.
pub mod auth;
pub mod database;
pub mod env_loader;
pub mod graceful_shutdown;
pub mod logger;
pub mod migrate;
pub mod service_container;
pub mod template;
