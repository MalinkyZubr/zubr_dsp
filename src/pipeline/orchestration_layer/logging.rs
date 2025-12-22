use env_logger;
pub use log::{debug, error, log_enabled, info, Level, warn, trace};
use std::sync::Once;


fn log_test_build() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn log_standard_build() {
    let _ = env_logger::init();
}

static INIT: Once = Once::new();

pub fn initialize_logger() {
    INIT.call_once(|| {
        if cfg!(test) {
            log_test_build();
        }
        else {
            log_standard_build();
        }
    })
}


pub fn log_message(message: String, log_level: Level) {
    if log_enabled!(log_level) {
        match log_level {
            Level::Info => info!("{}", message),
            Level::Debug => debug!("{}", message),
            Level::Warn => warn!("{}", message),
            Level::Error => error!("{}", message),
            Level::Trace => trace!("{}", message),
        }
    }
}