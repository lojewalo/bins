use bins::error::*;
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::PathBuf;
use toml::Value;

const DEFAULT_CONFIG_FILE: &'static str = r#"[defaults]
# If this is true, all pastes will be created as private or unlisted.
# Using the command-line option `--public` or `--private` will change this behavior.
private = true

# If this is true, all pastes will be made to accounts or with API keys defined in this file.
# Pastebin ignores this setting and the command-line argument, since Pastebin requires an API key to paste.
# Using the command-line option `--auth` or `--anon` will change this behavior.
auth = true

# Uncomment this line if you want to set a default service to use with bins. This will make the `--service` option
# optional and use the configured service if the option is not specified.
# service = ""

# If this is true, all commands will copy their output to the system clipboard.
# Using the command-line option `--copy` or `--no-copy` will change this behavior.
copy = false

[gist]
# The username to use for gist.github.com. This is ignored if access_token is empty.
username = ""

# Access token to use to log in to gist.github.com. If this is empty, an anonymous gist will be made.
# Generate a token from https://github.com/settings/tokens - only the gist permission is necessary
access_token = ""

[pastebin]
# The API key for pastebin.com. Learn more: http://pastebin.com/api
# If this is empty, all paste attempts to the pastebin service will fail.
api_key = ""

[hastebin]
# Uncomment this line if you want to set a default hastebin server besides `http://hastebin.com`.
# Using the command-line option `--server` will override this.
# server = ""

[bitbucket]
# The username for bitbucket.org.
username = ""

# The app password for bitbucket.org. Generate one in your Bitbucket settings under App passwords.
# Only the "snippets write" permission is necessary.
app_password = ""
"#;

pub struct BinsConfiguration {
  root: Value
}

impl BinsConfiguration {
  pub fn new() -> Result<Self> {
    let mut conf = BinsConfiguration { root: Value::Table(BTreeMap::new()) };
    conf.root = try!(conf.parse_config());
    Ok(conf)
  }

  pub fn get_defaults_private(&self) -> bool {
    self.root.lookup_bool_or("defaults.private", true)
  }

  pub fn get_defaults_auth(&self) -> bool {
    self.root.lookup_bool_or("defaults.auth", true)
  }

  pub fn get_defaults_service(&self) -> Option<&str> {
    self.root.lookup_str("defaults.service")
  }

  pub fn get_defaults_copy(&self) -> bool {
    self.root.lookup_bool_or("defaults.copy", false)
  }

  pub fn get_gist_username(&self) -> Option<&str> {
    self.root.lookup_str("gist.username")
  }

  pub fn get_gist_access_token(&self) -> Option<&str> {
    self.root.lookup_str("gist.access_token")
  }

  pub fn get_pastebin_api_key(&self) -> Option<&str> {
    self.root.lookup_str("pastebin.api_key")
  }

  pub fn get_hastebin_server(&self) -> Option<&str> {
    self.root.lookup_str("hastebin.server")
  }

  pub fn get_bitbucket_username(&self) -> Option<&str> {
    self.root.lookup_str("bitbucket.username")
  }

  pub fn get_bitbucket_app_password(&self) -> Option<&str> {
    self.root.lookup_str("bitbucket.app_password")
  }

  fn convert_size_str_to_bytes(string: &str) -> Result<u64> {
    let mut size: Vec<char> = Vec::new();
    let mut unit: Vec<char> = Vec::new();
    for c in string.trim().chars() {
      if "0123456789.".contains(c) {
        size.push(c);
      } else if "bBkKmMgGiI".contains(c) {
        unit.push(c);
      }
    }
    let size: f64 = match size.into_iter().collect::<String>().parse() {
      Ok(s) => s,
      Err(e) => return Err(e.to_string().into()),
    };
    let unit = unit.into_iter().collect::<String>().to_lowercase();
    let unit = if unit.is_empty() {
      1
    } else {
      match unit.as_str() {
        "b" => 1,
        "kb" => (10 as u64).pow(3),
        "kib" => (2 as u64).pow(10),
        "mb" => (10 as u64).pow(6),
        "mib" => (2 as u64).pow(20),
        "gb" => (10 as u64).pow(9),
        "gib" => (2 as u64).pow(30),
        _ => return Err(format!("invalid unit for max file size: \"{}\"", unit).into())
      }
    };
    Ok((size * unit as f64).round() as u64)
  }
}

trait Configurable {
  fn parse_config(&self) -> Result<Value>;

  fn get_config_paths(&self) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = vec![];
    if let Ok(dir) = env::var("XDG_CONFIG_DIR") {
      let mut xdg = PathBuf::from(dir);
      xdg.push("bins.cfg");
      paths.push(xdg);
    }
    let mut home = match env::home_dir() {
      Some(p) => p,
      None => return paths,
    };
    let mut dot_config = home.clone();
    dot_config.push(".config");
    dot_config.push("bins.cfg");
    paths.push(dot_config);
    home.push(".bins.cfg");
    paths.push(home);
    paths
  }

  fn get_config_path(&self) -> Option<PathBuf> {
    self.get_config_paths().into_iter().find(|p| p.exists())
  }
}

impl Configurable for BinsConfiguration {
  fn parse_config(&self) -> Result<Value> {
    let path = match self.get_config_path() {
      Some(p) => p,
      None => {
        let config_paths = self.get_config_paths();
        let priority = some_or_err!(config_paths.first(),
                                    "no possible config paths computed".into());
        let parent = some_or_err!(priority.parent(), "config file path had no parent".into());
        if !parent.exists() {
          let parent_str = some_or_err!(parent.to_str(),
                                        "config file path parent could not be converted to string".into());
          try!(fs::create_dir_all(parent_str));
        }
        if !parent.is_dir() || parent.is_file() {
          return Err("config file parent path was not a directory".into());
        }
        priority.to_path_buf()
      }
    };
    if !&path.exists() {
      let mut file = try!(File::create(&path));
      try!(file.write_all(DEFAULT_CONFIG_FILE.as_bytes()));
    }
    if (&path).is_dir() || !&path.is_file() {
      return Err("configuration file exists, but is not a valid file".into());
    }
    let mut config = String::new();
    try!(try!(File::open(path)).read_to_string(&mut config));
    match config.parse() {
      Ok(v) => Ok(v),
      Err(e) => {
        let message = e.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n");
        Err(format!("could not parse config (try making a backup and deleting it)\n\n{}",
                    message)
          .into())
      }
    }
  }
}

pub trait BetterLookups {
  fn lookup_str<'a>(&'a self, path: &'a str) -> Option<&str>;
  fn lookup_str_or<'a>(&'a self, key: &'a str, def: &'a str) -> &'a str;
  fn lookup_bool<'a>(&'a self, path: &'a str) -> Option<bool>;
  fn lookup_bool_or<'a>(&'a self, key: &'a str, def: bool) -> bool;
}

impl BetterLookups for Value {
  fn lookup_str<'a>(&'a self, path: &'a str) -> Option<&str> {
    match self.lookup(path) {
      Some(v) => v.as_str(),
      None => None,
    }
  }

  fn lookup_str_or<'a>(&'a self, key: &'a str, def: &'a str) -> &'a str {
    self.lookup_str(key).unwrap_or(def)
  }

  fn lookup_bool<'a>(&'a self, path: &'a str) -> Option<bool> {
    match self.lookup(path) {
      Some(v) => v.as_bool(),
      None => None,
    }
  }

  fn lookup_bool_or<'a>(&'a self, key: &'a str, def: bool) -> bool {
    self.lookup_bool(key).unwrap_or(def)
  }
}
