extern crate config;

use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use std::env;
use config::types::Config;
use bins::error::{BinsError, BinsErrorKind};

const DEFAULT_CONFIG_FILE: &'static str =
r#"gist = {
  /*
   * The username to use for gist.github.com. This is ignored if access_token is empty.
   */
  username = "";
  /*
   * Access token to use to log in to gist.github.com. If this is empty, an anonymous gist will be made.
   * Generate a token from https://github.com/settings/tokens - only the gist permission is necessary
   */
  access_token = "";
};

pastebin = {
  /*
   * The API key for pastebin.com. Learn more: http://pastebin.com/api
   * If this is empty, all paste attempts to the pastebin service will fail.
   */
  api_key = "";
};
"#;

pub struct BinsConfiguration;

impl BinsConfiguration {
  pub fn new() -> Self {
    BinsConfiguration { }
  }
}

pub trait Configurable {
  fn parse_config(&self) -> Result<Config, BinsError>;

  fn get_config_path(&self) -> Option<PathBuf> {
    let mut home = match env::home_dir() {
      Some(p) => p,
      None => return None
    };
    home.push(".bins.cfg");
    Some(home)
  }
}

impl Configurable for BinsConfiguration {
  fn parse_config(&self) -> Result<Config, BinsError> {
    let path = match self.get_config_path() {
      Some(p) => p,
      None => return Err(
        BinsError {
          kind: BinsErrorKind::None,
          message: String::from("could not get path to the configuration file")
        }
      )
    };
    if !&path.exists() {
      let mut file = try!(File::create(&path));
      try!(file.write_all(DEFAULT_CONFIG_FILE.as_bytes()));
    }
    if (&path).is_dir() || !&path.is_file() {
      return Err(BinsError::from("configuration file exists, but is not a valid file"))
    }
    config::reader::from_file(path.as_path()).map_err(|e| BinsError::from(e))
  }
}
