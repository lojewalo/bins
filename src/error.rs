use url::ParseError;
use hyper::error::Error as HyperError;
use serde_json::error::Error as JsonError;
use toml::de::Error as TomlError;

use std::io::Error as IoError;
use std::any::Any;

error_chain! {
  foreign_links {
    Http(HyperError);
    UrlParse(ParseError);
    Io(IoError);
    Json(JsonError);
    Toml(TomlError);
    Magic(::magic::MagicError) #[cfg(feature = "file_type_checking")];
  }

  errors {
    BadRangeNumber(parse_error: ::std::num::ParseIntError) {
      description("an invalid number was used in a range")
      display("bad number in range: {}", parse_error)
    }
    BadRange {
      description("a range had too many components")
      display("range had too many components")
    }
    Thread(inside: Box<Any + Send + 'static>) {
      description("a thread panicked")
      display("a thread panicked")
    }
    InvalidResponse {
      description("the bin responded incorrectly (or updated with a breaking change)")
      display("the bin responded incorrectly (or updated with a breaking change)")
    }
    InvalidStatus(code: u16, message: Option<String>) {
      description("the bin responded with an incorrect status code")
      display("the bin responded with an invalid status ({}){}",
        code,
        message.as_ref().map(|m| format!("\nthe bin also included this content with the error:\n\n{}", m)).unwrap_or_default())
    }
    BinError(message: String) {
      display("{}", message)
    }
    #[cfg(feature = "file_type_checking")]
    InvalidFileType(name: String, kind: String) {
      description("an invalid file type was used as an input")
      display("bins stopped before uploading because {} is a disallowed file type ({})", name, kind)
    }
    Config {
      description("bins could not find a configuration file, and it was impossible to create one")
      display("bins could not find a configuration file, and it was impossible to create one")
    }
    Other {
      description("an error occurred. please let us know so we can provide a better error message")
      display("an error occurred. please let us know so we can provide a better error message")
    }
  }
}
