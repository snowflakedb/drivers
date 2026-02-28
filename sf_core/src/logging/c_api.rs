use crate::logging;

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init_logger(callback: logging::CLogCallback) -> u32 {
    let config = logging::LoggingConfig::new(None, false, false);
    let layer = logging::CallbackLayer::new(callback);
    match logging::init_logging(config, Some(layer)) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            1
        }
    }
}
