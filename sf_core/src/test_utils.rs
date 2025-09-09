/// Sets up logging for tests
#[cfg(test)]
pub fn setup_logging() {
    use tracing::Level;
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env()
        .unwrap();
    let _ = tracing_subscriber::fmt::fmt()
        .with_env_filter(env_filter)
        .try_init();
}
