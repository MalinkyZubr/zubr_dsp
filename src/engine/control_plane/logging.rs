use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use std::sync::{Arc, OnceLock};
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering::{Acquire, Release};
use scc::Queue;


static LOG_LEVEL: OnceLock<Arc<AtomicU8>> = OnceLock::new();
static LOG_QUEUE: OnceLock<Arc<Queue<String>>> = OnceLock::new();
const LOG_QUEUE_SIZE: usize = 1024;


pub fn level_from_u8(val: u8) -> Result<Level, &'static str> {
    match val {
        1 => Ok(Level::Error),
        2 => Ok(Level::Warn),
        3 => Ok(Level::Info),
        4 => Ok(Level::Debug),
        5 => Ok(Level::Trace),
        _ => Err("Invalid log level u8 value"),
    }
}


pub fn SET_LOG_LEVEL(level: LevelFilter) {
    log::set_max_level(level);
    let current_level = LOG_LEVEL.get_or_init(|| Arc::new(AtomicU8::new(1)));
    current_level.store(level as u8, Release);
}
pub fn GET_LOG_LEVEL() -> Level {
    let current_level = LOG_LEVEL.get_or_init(|| Arc::new(AtomicU8::new(1)));
    let level = current_level.load(Acquire);
    level_from_u8(level).unwrap()
}
pub fn LOG_QUEUE_PUSH(msg: String) {
    let queue = LOG_QUEUE.get_or_init(|| Arc::new(Queue::new()));

    while queue.len() > LOG_QUEUE_SIZE {
        queue.pop();
    }
    queue.push(msg);
}


pub fn LOG_QUEUE_POP_ALL() -> Vec<String> {
    let queue = LOG_QUEUE.get_or_init(|| Arc::new(Queue::new()));

    let mut output_vec = Vec::with_capacity(queue.len());
    while !queue.is_empty() {
        let value = queue.pop();
        match value {
            Some(msg) => output_vec.push((**msg).clone()),
            None => ()
        }
    }

    output_vec
}

/// Trait for different output destinations (file, stdout, socket, etc.)
pub trait LogOutput: Send + Sync {
    /// Write a formatted log message to the output destination
    fn write(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Flush any buffered output (optional, default implementation does nothing)
    fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

/// Global logger implementation
pub struct GlobalLogger {
    outputs: Vec<Arc<dyn LogOutput>>,
}

impl GlobalLogger {
    /// Create a new GlobalLogger with the specified output and level
    pub fn new(outputs: Vec<Arc<dyn LogOutput>>, level: Level) -> Self {
        Self { outputs }
    }

    /// Format a log record into a string
    fn format_record(&self, record: &Record) -> String {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
        format!(
            "[{}] [{}] [{}:{}] {}",
            timestamp,
            record.level(),
            record.file().unwrap_or("unknown"),
            record.line().unwrap_or(0),
            record.args()
        )
    }
}

impl Log for GlobalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= GET_LOG_LEVEL()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let formatted_message = self.format_record(record);
            for output in self.outputs.iter() {
                if let Err(e) = output.write(&formatted_message) {
                    eprintln!("Failed to write log: {}", e);
                }
            }
        }
    }

    fn flush(&self) {
        for output in self.outputs.iter() {
            if let Err(e) = output.flush() {
                eprintln!("Failed to flush log output: {}", e);
            }
        }
    }
}

/// Concrete implementation for stdout logging
pub struct StdoutOutput;
impl StdoutOutput {
    pub fn new() -> Self {
        Self
    }
}

impl LogOutput for StdoutOutput {
    fn write(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("{}", message);
        Ok(())
    }

    fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::io::{self, Write};
        io::stdout().flush()?;
        Ok(())
    }
}


pub struct QueueOutput;
impl QueueOutput {
    pub fn new() -> Self {
        Self {}
    }
}

impl LogOutput for QueueOutput {
    fn write(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        LOG_QUEUE_PUSH(message.to_string());

        Ok(())
    }
}

/// Initialize the global logger with the specified output and level
pub fn init_logger(outputs: Vec<Arc<dyn LogOutput>>, level: Level) -> Result<(), SetLoggerError> {
    let logger = GlobalLogger::new(outputs, level);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}

/// Convenience function to initialize stdout logger with specified level
pub fn init_stdout_logger(level: Level) -> Result<(), SetLoggerError> {
    let stdout_output = Arc::new(StdoutOutput::new());
    init_logger(vec![stdout_output], level)
}

pub fn init_queue_logger(level: Level) -> Result<(), SetLoggerError> {
    let queue_output = Arc::new(QueueOutput::new());
    init_logger(vec![queue_output], level)
}

pub fn init_full_logger(level: Level) -> Result<(), SetLoggerError> {
    let stdout_output = Arc::new(StdoutOutput::new());
    let queue_output = Arc::new(QueueOutput::new());

    init_logger(vec![stdout_output, queue_output], level)
}


/// Convenience function to initialize stdout logger with info level
pub fn init_default_logger() -> Result<(), SetLoggerError> {
    init_full_logger(Level::Info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    // Mock output for testing
    struct MockOutput {
        messages: Arc<Mutex<VecDeque<String>>>,
    }

    impl MockOutput {
        fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        fn get_messages(&self) -> Vec<String> {
            self.messages.lock().unwrap().drain(..).collect()
        }
    }

    impl LogOutput for MockOutput {
        fn write(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.messages.lock().unwrap().push_back(message.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_stdout_output() {
        let output = StdoutOutput::new();
        // This would print to stdout in a real scenario
        assert!(output.write("test message").is_ok());
        assert!(output.flush().is_ok());
    }

    #[test]
    fn test_logger_level_filtering() {
        let mock_output = Arc::new(MockOutput::new());
        let logger = GlobalLogger::new(vec![mock_output.clone()], Level::Warn);

        // Test that debug messages are filtered out
        let debug_record = log::Record::builder()
            .level(Level::Debug)
            .args(format_args!("debug message"))
            .file(Some("test.rs"))
            .line(Some(1))
            .build();

        assert!(!logger.enabled(debug_record.metadata()));

        // Test that error messages are allowed
        let error_record = log::Record::builder()
            .level(Level::Error)
            .args(format_args!("error message"))
            .file(Some("test.rs"))
            .line(Some(1))
            .build();

        assert!(logger.enabled(error_record.metadata()));
    }

    #[test]
    fn test_logger_formatting() {
        let mock_output = Arc::new(MockOutput::new());
        let logger = GlobalLogger::new(vec![mock_output.clone()], Level::Debug);

        let record = log::Record::builder()
            .level(Level::Info)
            .args(format_args!("test message"))
            .file(Some("test.rs"))
            .line(Some(42))
            .build();

        logger.log(&record);

        let messages = mock_output.get_messages();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("INFO"));
        assert!(messages[0].contains("test message"));
        assert!(messages[0].contains("test.rs:42"));
    }

    #[test]
    fn test_init_default_logger() {
        // Reset logger for testing
        // Note: In a real application, you'd typically only initialize once
        assert!(init_default_logger().is_ok());

        // Test that logging works
        log::info!("This is a test log message");
        log::debug!("This debug message should not appear with info level");
        log::error!("This error message should appear");
    }
}
