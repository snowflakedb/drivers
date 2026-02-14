use std::sync::OnceLock;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Get the global async runtime, initializing on first access.
///
/// # Notes
/// The returned runtime must not be used with `block_on` from within
/// an existing tokio runtime context (e.g. `#[tokio::test]`), as that
/// will panic.
pub(crate) fn runtime() -> Result<&'static tokio::runtime::Runtime, std::io::Error> {
    if let Some(rt) = RUNTIME.get() {
        return Ok(rt);
    }
    // Build may be called more than once if multiple threads race here,
    // but `get_or_init` guarantees only one value is stored; the extra
    // runtime is simply dropped.
    let rt = build()?;
    Ok(RUNTIME.get_or_init(|| rt))
}

fn build() -> Result<tokio::runtime::Runtime, std::io::Error> {
    // Multi-thread is required: reqwest/hyper spawn background tasks
    // that must make progress during block_on.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("sf-core-async-bridge-worker")
        .build()
}
