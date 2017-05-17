use log::{set_logger, Log, LogRecord, LogLevel, LogLevelFilter, LogMetadata, SetLoggerError};
use std::io::{Write, stderr};
use time::now;

pub struct Logger {
  max_level: LogLevel
}

impl Logger {
  pub fn new(max_level: LogLevel) -> Self {
    Logger {
      max_level: max_level
    }
  }

  pub fn init(self) -> Result<(), SetLoggerError> {
    set_logger(|max_log_level| {
      max_log_level.set(LogLevelFilter::Debug);
      Box::new(self)
    })
  }
}

impl Log for Logger {
  fn enabled(&self, metadata: &LogMetadata) -> bool {
    metadata.level() <= self.max_level
  }

  fn log(&self, record: &LogRecord) {
    if self.enabled(record.metadata()) && !record.target().contains("rustls") {
      let time_now = now();
      let time = time_now.strftime("%H:%M:%S").unwrap();
      writeln!(stderr(), "[{}] {} – {}", time, record.level(), record.args()).unwrap();
    }
  }
}
