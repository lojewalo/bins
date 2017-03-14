use url::ParseError;
use hyper::error::Error as HyperError;
use serde_json::error::Error as JsonError;
use toml::de::Error as TomlError;

use std::io::Error as IoError;
use std::result::Result as StdResult;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::fmt::Result as FmtResult;
use std::any::Any;

pub type Result<T> = StdResult<T, BinsError>;

#[derive(Debug)]
pub enum BinsError {
  Http(HyperError),
  UrlParse(ParseError),
  Io(IoError),
  Json(JsonError),
  Toml(TomlError),
  Thread(Box<Any + Send + 'static>),

  InvalidResponse,
  InvalidStatus(u16, Option<String>),
  /// An error reported by the bin after attempting an upload.
  BinError(String),

  UnsupportedFeature,
  Config,
  Other
}

impl Display for BinsError {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    if let BinsError::BinError(ref s) = *self {
      write!(f, "the bin responded with the following error: {}", s)
    } else if let BinsError::InvalidStatus(code, ref s) = *self {
      let content = s.clone().unwrap_or_default();
      write!(f, "the bin responded with an invalid status ({}) {}", code, content)
    } else {
      write!(f, "{}", self.description())
    }
  }
}

impl StdError for BinsError {
  fn description(&self) -> &str {
    match *self {
      BinsError::Http(ref e) => e.description(),
      BinsError::UrlParse(ref e) => e.description(),
      BinsError::Io(ref e) => e.description(),
      BinsError::Json(ref e) => e.description(),
      BinsError::Toml(ref e) => e.description(),
      BinsError::Thread(_) => "a thread panicked",
      BinsError::InvalidResponse => "the bin responded incorrectly (or updated with a breaking change)",
      BinsError::InvalidStatus(_, _) => "the bin responded with an incorrect status code",
      BinsError::BinError(ref s) => s,
      BinsError::UnsupportedFeature => "bins stopped because an unsupported feature was used with the selected bin",
      BinsError::Config => "bins could not find a configuration file, and it was impossible to create one",
      BinsError::Other => "an error occurred. please let us know so we can provide a better error message"
    }
  }

  fn cause(&self) -> Option<&StdError> {
    match *self {
      BinsError::Http(ref e) => Some(e),
      BinsError::UrlParse(ref e) => Some(e),
      BinsError::Io(ref e) => Some(e),
      BinsError::Json(ref e) => Some(e),
      BinsError::Toml(ref e) => Some(e),
      _ => None
    }
  }
}
