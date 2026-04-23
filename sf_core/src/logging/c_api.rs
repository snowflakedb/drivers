use tracing::level_filters::LevelFilter;

use crate::logging;
use crate::logging::LogManager;

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init_logger(callback: logging::CLogCallback) -> u32 {
    let config = logging::LoggingConfig::new(None, false, false);
    match LogManager::init(config) {
        Ok(mgr) => {
            mgr.subscribe_wrapper(callback, LevelFilter::INFO);
            0
        }
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            1
        }
    }
}
