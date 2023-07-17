use std::io::Write;

pub fn init_logger() {
    env_logger::Builder::new()
        .format(
            |buf: &mut env_logger::fmt::Formatter, record: &log::Record| {
                writeln!(
                    buf,
                    "{} [{}] {}:{} - {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    record.file().expect("no file"),
                    record.line().expect("no line"),
                    record.args()
                )
            },
        )
        .filter(None, log::LevelFilter::Info)
        .init();
}
