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

use BinFeature;

pub type Result<T> = StdResult<T, BinsError>;

#[derive(Debug)]
pub enum BinsError {
  Http(HyperError),
  UrlParse(ParseError),
  Io(IoError),
  Json(JsonError),
  Toml(TomlError),
  #[cfg(feature = "file_type_checking")]
  Magic(::magic::MagicError),
  Thread(Box<Any + Send + 'static>),

  InvalidResponse,
  InvalidStatus(u16, Option<String>),
  /// An error reported by the bin after attempting an upload.
  BinError(String),

  Main(MainError),
  #[cfg(feature = "file_type_checking")]
  InvalidFileType {
    name: String,
    kind: String
  },
  Config,
  Other
}

impl Display for BinsError {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    match *self {
      BinsError::BinError(ref s) => write!(f, "the bin responded with the following error: {}", s),
      BinsError::InvalidStatus(code, ref s) => match *s {
        Some(ref string) => write!(f, "the bin responded with an invalid status ({})\nthe bin also included this content with the error:\n\n{}", code, string),
        None => write!(f, "the bin responded with an invalid status ({})", code)
      },
      BinsError::Main(ref e) => write!(f, "{}", e.to_string()),
      #[cfg(feature = "file_type_checking")]
      BinsError::InvalidFileType { ref name, ref kind } => write!(f, "bins stopped before uploading because {} is a disallowed file type ({})", name, kind),
      _ => write!(f, "{}", self.description())
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
      #[cfg(feature = "file_type_checking")]
      BinsError::Magic(ref e) => e.description(),
      BinsError::Thread(_) => "a thread panicked",
      BinsError::InvalidResponse => "the bin responded incorrectly (or updated with a breaking change)",
      BinsError::InvalidStatus(_, _) => "the bin responded with an incorrect status code",
      BinsError::BinError(ref s) => s,
      BinsError::Main(_) => "an error in the main function set",
      #[cfg(feature = "file_type_checking")]
      BinsError::InvalidFileType { .. } => "an invalid file type was used as an input",
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
      #[cfg(feature = "file_type_checking")]
      BinsError::Magic(ref e) => Some(e),
      _ => None
    }
  }
}

#[derive(Debug)]
pub enum MainError {
  NoSuchBin(String),
  NoBinSpecified,
  UnsupportedFeature(String, BinFeature),
  ParseId,
  NameWithMultipleFiles,
  InvalidSizeLimit,
  FileOverSizeLimit {
    name: String,
    size: u64,
    limit: u64
  }
}

impl MainError {
  fn to_string(&self) -> String {
    match *self {
      MainError::NoSuchBin(ref bin) => format!("there is no bin called \"{}\"", bin),
      MainError::NoBinSpecified => String::from("no bin was specified"),
      MainError::UnsupportedFeature(ref bin, ref feat) => format!("bins stopped because {} does not support {} pastes", bin, feat),
      MainError::ParseId => String::from("could not parse ID from HTML URL"),
      MainError::NameWithMultipleFiles => String::from("cannot use --name with multiple upload files"),
      MainError::InvalidSizeLimit => String::from("the file size limit specified in the config is invalid"),
      MainError::FileOverSizeLimit { ref name, size, limit } => format!("{} is {} byte{}, which is over the size limit of {} byte{}",
        name,
        size,
        if size == 1 { "" } else { "s" },
        limit,
        if limit == 1 { "" } else { "s" })
    }
  }
}
