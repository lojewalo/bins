extern crate config;

use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use std::env;
use config::types::Config;
use bins::error::*;

const DEFAULT_CONFIG_FILE: &'static str =
r#"defaults = {
  /*
   * If this is true, all pastes will be created as private or unlisted.
   * Using the command-line option `--public` or `--private` will change this behavior.
   */
  private = true;
  /*
   * If this is true, all pastes will be made to accounts or with API keys defined in this file.
   * Pastebin ignores this setting and the command-line argument, since Pastebin requires an API key to paste.
   * Using the command-line option `--auth` or `--anon` will change this behavior.
   */
  auth = true;
};

gist = {
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
  fn parse_config(&self) -> Result<Config>;

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
  fn parse_config(&self) -> Result<Config> {
    let path = match self.get_config_path() {
      Some(p) => p,
      None => return Err("could not get path to the configuration file".into())
    };
    if !&path.exists() {
      let mut file = try!(File::create(&path));
      try!(file.write_all(DEFAULT_CONFIG_FILE.as_bytes()));
    }
    if (&path).is_dir() || !&path.is_file() {
      return Err("configuration file exists, but is not a valid file".into())
    }
    Ok(try!(config::reader::from_file(path.as_path())))
  }
}
