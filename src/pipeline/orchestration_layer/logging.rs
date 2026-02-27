use log::{Level, Log, Metadata, Record, SetLoggerError};
use std::sync::Arc;

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
    output: Arc<dyn LogOutput>,
    level: Level,
}

impl GlobalLogger {
    /// Create a new GlobalLogger with the specified output and level
    pub fn new(output: Arc<dyn LogOutput>, level: Level) -> Self {
        Self { output, level }
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
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let formatted_message = self.format_record(record);
            if let Err(e) = self.output.write(&formatted_message) {
                eprintln!("Failed to write log: {}", e);
            }
        }
    }

    fn flush(&self) {
        if let Err(e) = self.output.flush() {
            eprintln!("Failed to flush log output: {}", e);
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

/// Initialize the global logger with the specified output and level
pub fn init_logger(
    output: Arc<dyn LogOutput>,
    level: Level,
) -> Result<(), SetLoggerError> {
    let logger = GlobalLogger::new(output, level);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}

/// Convenience function to initialize stdout logger with specified level
pub fn init_stdout_logger(level: Level) -> Result<(), SetLoggerError> {
    let stdout_output = Arc::new(StdoutOutput::new());
    init_logger(stdout_output, level)
}

/// Convenience function to initialize stdout logger with info level
pub fn init_default_logger() -> Result<(), SetLoggerError> {
    init_stdout_logger(Level::Info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::collections::VecDeque;

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
        let logger = GlobalLogger::new(mock_output.clone(), Level::Warn);
        
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
        let logger = GlobalLogger::new(mock_output.clone(), Level::Debug);
        
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