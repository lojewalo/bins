extern crate std;
extern crate config;

use std::fmt::{self, Display};
use config::error::ConfigError;

#[derive(Debug)]
pub struct BinsError {
  pub kind: BinsErrorKind,
  pub message: String
}

impl Display for BinsError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl std::error::Error for BinsError {
  fn description(&self) -> &str {
    &self.message
  }
}

impl From<ConfigError> for BinsError {
  fn from (e: ConfigError) -> Self {
    let msg = format!("{}", e);
    BinsError {
      kind: BinsErrorKind::ConfigError(e),
      message: format!("{}", msg)
    }
  }
}

impl From<std::io::Error> for BinsError {
  fn from(e: std::io::Error) -> Self {
    let msg = format!("{}", e);
    BinsError {
      kind: BinsErrorKind::IoError(e),
      message: format!("{}", msg)
    }
  }
}

impl From<String> for BinsError {
  fn from(e: String) -> Self {
    BinsError {
      kind: BinsErrorKind::None,
      message: e
    }
  }
}

impl<'a> From<&'a str> for BinsError {
  fn from(e: &'a str) -> Self {
    BinsError {
      kind: BinsErrorKind::None,
      message: String::from(e)
    }
  }
}

#[derive(Debug)]
pub enum BinsErrorKind {
  None,
  ConfigError(config::error::ConfigError),
  IoError(std::io::Error)
}
