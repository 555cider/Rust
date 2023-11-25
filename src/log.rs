use crate::constant::{TRACING_WITH_FILE, TRACING_WITH_LINE_NUMBER, TRACING_WITH_THREAD_IDS};

pub fn init_logging() {
    tracing_subscriber::fmt()
        .compact() // Use a more compact, abbreviated log format
        .with_file(TRACING_WITH_FILE) // Display source code file paths
        .with_line_number(TRACING_WITH_LINE_NUMBER) // Display source code line numbers
        .with_thread_ids(TRACING_WITH_THREAD_IDS) // Display the thread ID an event was recorded on
        .with_target(false) // Display the event's target (module path)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_writer(std::io::stdout)
        .init();
}
