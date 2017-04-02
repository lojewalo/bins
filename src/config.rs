use range::BidirectionalRange;

pub const DEFAULT_CONFIG_GZIP: &'static [u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/bins.cfg.gz"));

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
  pub general: ConfigGeneral,
  pub safety: ConfigSafety,
  pub defaults: ConfigDefaults,
  pub gist: ConfigGist,
  pub pastebin: ConfigPastebin,
  pub hastebin: ConfigHastebin,
  pub bitbucket: ConfigBitbucket
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigGeneral {
  pub file_size_limit: Option<String>
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigSafety {
  pub disallowed_file_patterns: Option<Vec<String>>,
  pub disallowed_file_types: Option<Vec<String>>,
  pub cancel_on_unsupported: Option<bool>,
  pub warn_on_unsupported: Option<bool>
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigDefaults {
  pub private: Option<bool>,
  pub authed: Option<bool>,
  pub bin: Option<String>,
  pub copy: Option<bool>
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigGist {
  pub username: Option<String>,
  pub access_token: Option<String>
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigPastebin {
  pub api_key: Option<String>
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigHastebin {
  pub server: Option<String>
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigBitbucket {
  pub username: Option<String>,
  pub app_password: Option<String>
}

#[derive(Debug, Default)]
pub struct CommandLineOptions {
  pub authed: Option<bool>,
  pub private: Option<bool>,
  pub file_name: Option<String>,
  pub json: Option<bool>,
  pub url_output: Option<UrlOutputMode>,
  pub force: Option<bool>,
  pub name: Option<String>,
  #[cfg(feature = "clipboard_support")]
  pub copy: Option<bool>,
  pub list_all: Option<bool>,
  pub range: Option<Vec<BidirectionalRange<usize>>>
}

impl CommandLineOptions {
  pub fn json(&self) -> bool {
    match self.json {
      Some(true) => true,
      _ => false
    }
  }
}

#[derive(Debug)]
pub enum UrlOutputMode {
  Html,
  Raw
}
