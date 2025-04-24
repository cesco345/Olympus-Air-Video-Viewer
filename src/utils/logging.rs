// src/utils/logging.rs
use log::LevelFilter;

/// Initialize application logging with INFO level
pub fn init() {
    env_logger::builder().filter_level(LevelFilter::Info).init();
}

/// Initialize application logging with WARNING level only
/// This reduces console output for better UI experience
pub fn init_quiet() {
    env_logger::builder().filter_level(LevelFilter::Warn).init();
}
