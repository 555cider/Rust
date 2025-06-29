use crate::config::env_loader::AppConfig;
use std::{fs::create_dir_all, io::stdout, path::Path};
use tracing::{debug, info};
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer, Registry,
};

/// Initialize the global logger with configured sinks.
///
/// # Panics
/// Panics if the logger is already initialized.
pub fn init_logger(config: &AppConfig) {
    let log_config = &config.log;
    let mut layers = Vec::<Box<dyn Layer<Registry> + Send + Sync>>::new();
    let mut guards = Vec::new();

    // Configure console logging
    if log_config.console_enable {
        let (console_writer, guard) = non_blocking(stdout());
        let console_layer = create_console_layer(console_writer, &log_config.console_level);
        layers.push(Box::new(console_layer));
        guards.push(guard);
    }

    // Configure file logging with daily rotation
    if log_config.file_enable {
        if let Some(parent) = Path::new(&log_config.file_path).parent() {
            if let Err(e) = create_dir_all(parent) {
                tracing::error!("로그 디렉토리 생성 실패: {}", e);
            }
        }

        match rolling::Builder::new()
            .rotation(rolling::Rotation::DAILY)
            .filename_prefix("app")
            .build(&log_config.file_path)
        {
            Ok(file_appender) => {
                let (file_writer, guard) = non_blocking(file_appender);
                let file_layer = create_file_layer(file_writer, &log_config.file_level);
                layers.push(Box::new(file_layer));
                guards.push(guard);
                tracing::debug!("파일 로거 초기화 완료: {}", log_config.file_path);
            }
            Err(e) => {
                tracing::error!(
                    "파일 로거 초기화 실패 - 경로: {}, 오류: {}",
                    log_config.file_path,
                    e
                );
            }
        }
    }

    // Initialize the global logger with all configured layers
    let subscriber = Registry::default();

    if !layers.is_empty() {
        let combined_layer = layers
            .into_iter()
            .reduce(|acc, layer| {
                Box::new(acc.and_then(layer)) as Box<dyn Layer<Registry> + Send + Sync>
            })
            .unwrap();

        subscriber.with(combined_layer).init();
    } else {
        subscriber.init();
    }

    // Prevent guards from being dropped
    std::mem::forget(guards);

    // Log initialization status
    let console_status = if log_config.console_enable {
        format!("활성화 (레벨: {})", log_config.console_level)
    } else {
        "비활성화".to_string()
    };

    let file_status = if log_config.file_enable {
        format!(
            "활성화 (경로: {}, 레벨: {})",
            log_config.file_path, log_config.file_level
        )
    } else {
        "비활성화".to_string()
    };

    info!(
        "로거 초기화 완료 - 콘솔: {}, 파일: {}",
        console_status, file_status
    );
    debug!("자세한 디버그 로그가 활성화되었습니다.");
}

/// Creates a console logging layer with ANSI colors and source location.
///
/// # Arguments
/// * `writer` - The writer to use for log output
/// * `level` - The minimum log level to display
fn create_console_layer<W>(writer: W, level: &str) -> impl Layer<Registry> + Send + Sync + 'static
where
    W: for<'writer> fmt::MakeWriter<'writer> + 'static + Send + Sync,
{
    let env_filter = EnvFilter::try_new(level)
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,axum=info"));

    fmt::layer()
        .with_writer(writer)
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_filter(env_filter)
}

/// Creates a file logging layer with thread IDs and timestamps.
///
/// # Arguments
/// * `writer` - The writer to use for log output
/// * `level` - The minimum log level to write to file
fn create_file_layer<W>(writer: W, level: &str) -> impl Layer<Registry> + Send + Sync + 'static
where
    W: for<'writer> fmt::MakeWriter<'writer> + 'static + Send + Sync,
{
    let env_filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));

    let layer = fmt::Layer::new()
        .with_writer(writer)
        .with_ansi(false)
        .with_thread_ids(true)
        .with_level(true)
        .with_target(true);

    layer.with_filter(env_filter)
}
