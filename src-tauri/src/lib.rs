mod app;
mod errors;
mod models;
mod platform;
mod session;
mod settings;
mod shell;

fn init_logging() {
    let level = if cfg!(debug_assertions) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::WARN
    };
    let _ = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
}

pub fn run() {
    init_logging();
    app::run();
}
