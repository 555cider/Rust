mod config;
mod errors;
mod filter;
mod handler;
mod model;
mod repository;
mod service;
mod util;

use crate::config::{
    database, env_loader::AppConfig, graceful_shutdown, logger, migrate,
    service_container::ServiceContainer, template,
};
use anyhow::Context;
use axum::{extract::FromRef, http::HeaderName, middleware, Router};
use std::{net::SocketAddr, sync::Arc};
use tera::Tera;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{self, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
};

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: sqlx::SqlitePool,
    pub tera: Arc<Tera>,
    pub service: ServiceContainer,
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to initialize tokio runtime")?;

    if let Err(e) = rt.block_on(async_main()) {
        eprintln!("Application error: {:#?}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn async_main() -> anyhow::Result<()> {
    // Load configuration
    let config = AppConfig::from_env();

    // Initialize logger with configuration
    logger::init_logger(&config);
    tracing::info!("🚀 어드민 서버를 시작합니다.");

    // Run database migrations
    tracing::info!("🔄 데이터베이스 마이그레이션을 실행합니다...");
    migrate::run_migrations(&config.database_url)
        .await
        .context("데이터베이스 마이그레이션 실패")?;
    tracing::info!("✅ 데이터베이스 마이그레이션이 완료되었습니다.");

    // Initialize database connection pool
    tracing::debug!("데이터베이스 연결 풀을 초기화합니다.");
    let db_pool = database::setup_db_pool(&config.database_url).await;

    // Initialize template engine
    tracing::debug!("템플릿 엔진을 초기화합니다.");
    let tera = match template::setup_tera() {
        Ok(tera) => tera,
        Err(e) => {
            tracing::warn!("기본 템플릿 로딩 실패: {}", e);
            tracing::info!("대체 템플릿 로더를 시도합니다...");
            template::setup_tera_with_fallback().context("템플릿 엔진 초기화 실패")?
        }
    };

    let template_count = tera.get_template_names().count();
    tracing::debug!("로드된 템플릿: {}개", template_count);
    let tera_arc = Arc::new(tera);

    // Initialize service container
    let service = ServiceContainer::new(Arc::from(db_pool.clone()));

    // Create application state
    let app_state = AppState {
        config,
        pool: db_pool,
        tera: tera_arc,
        service,
    };

    // Configure router and middleware
    let app = Router::new()
        .merge(handler::route())
        .with_state(app_state.clone())
        .layer(
            ServiceBuilder::new()
                .layer(
                    ServiceBuilder::new()
                        .layer(SetRequestIdLayer::new(
                            HeaderName::from_static("x-request-id"),
                            request_id::MakeRequestUuid,
                        ))
                        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
                            "x-request-id",
                        )))
                        .into_inner(),
                )
                .layer(SetSensitiveRequestHeadersLayer::new(vec![
                    HeaderName::from_static("authorization"),
                ]))
                .layer(middleware::from_fn(filter::optional_auth))
                .layer(middleware::from_fn(filter::log)),
        );

    // Start HTTP server
    let addr = SocketAddr::from(([0, 0, 0, 0], app_state.config.server_port));
    tracing::info!("🌐 서버 시작 중: http://{}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .context("서버 소켓 바인딩 실패")?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(graceful_shutdown::shutdown_signal())
    .await?;

    Ok(())
}
