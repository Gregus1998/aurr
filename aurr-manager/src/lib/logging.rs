//Module to handle all of the logging. Written by claudia Haiku 4.5. Pretty good and simple.
use std::fs::File;
use std::sync::Mutex;
use std::io::Write;
use chrono::{self, Local};

/// Global logger instance
pub static LOGGER: once_cell::sync::Lazy<Logger> = 
    once_cell::sync::Lazy::new(Logger::new);

/// Global log file path - set this before initializing the logger
pub static LOG_FILE_PATH: Mutex<Option<String>> = Mutex::new(None);

/// Logger struct that handles all logging operations
pub struct Logger {
    log_file: Mutex<Option<File>>,
}

impl Logger {
    /// Create a new logger instance
    pub fn new() -> Self {
        Logger {
            log_file: Mutex::new(None),
        }
    }

    /// Initialize the logger with colored terminal output and file logging
    pub fn init(log_file_path: Option<String>) {
    
        // Set the global log file path with datetime
        if let Some(path) = log_file_path {
            // Format: path/to/file_YYYY-MM-DD_HH-MM-SS.log
            let now = Local::now();
            let datetime_str = now.format("%Y-%m-%d_%H-%M-%S").to_string();
            
            // Insert datetime before the file extension
            let final_path = if path.contains('.') {
                let parts: Vec<&str> = path.rsplitn(2, '.').collect();
                format!("{}_{}.{}", parts[1], datetime_str, parts[0])
            } else {
                format!("{}_{}", path, datetime_str)
            };
            
            if let Ok(mut log_path) = LOG_FILE_PATH.lock() {
                *log_path = Some(final_path.clone());
            }

            // Open the log file for writing
            if let Ok(file) = File::create(&final_path) {
                if let Ok(mut log_file) = LOGGER.log_file.lock() {
                    *log_file = Some(file);
                }
            }
        }

        // Set up tracing subscriber with colored output
        tracing_subscriber::fmt()
            .with_ansi(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .init();
    }

    /// Write to log file if configured
    fn write_to_file(message: &str) {
        if let Ok(mut log_file) = LOGGER.log_file.lock() {
            if let Some(ref mut file) = *log_file {
                let _ = writeln!(file, "{}", message);
                let _ = file.flush();
            }
        }
    }

    /// Log an info message
    pub fn info(msg: &str) {
        tracing::info!("{}", msg);
        Self::write_to_file(&format!("[INFO] {}", msg));
    }

    /// Log a debug message
    pub fn debug(msg: &str) {
        tracing::debug!("{}", msg);
        Self::write_to_file(&format!("[DEBUG] {}", msg));
    }

    /// Log an error message
    pub fn error(msg: &str) {
        tracing::error!("{}", msg);
        Self::write_to_file(&format!("[ERROR] {}", msg));
    }

    /// Log a warning message
    pub fn warning(msg: &str) {
        tracing::warn!("{}", msg);
        Self::write_to_file(&format!("[WARN] {}", msg));
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenient macros for logging
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::lib::logging::Logger::info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::lib::logging::Logger::debug(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::lib::logging::Logger::error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {
        $crate::lib::logging::Logger::warning(&format!($($arg)*))
    };
}