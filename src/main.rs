#![feature(box_syntax)]

extern crate bins as lib;
extern crate hyper;
extern crate hyper_openssl;
extern crate url;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate clap;
extern crate toml;
extern crate flate2;
#[macro_use]
extern crate log;
extern crate time;
#[cfg(feature = "file_type_checking")]
extern crate magic;
#[cfg(feature = "clipboard_support")]
extern crate clipboard;
extern crate rand;
extern crate base64;

macro_rules! option {
  ($e: expr) => {{
    match $e {
      Some(x) => x,
      None => return None
    }
  }}
}

// TODO: refactor Bins::download
// TODO: move loose functions into Bins
// TODO: refactor inner
// TODO: investigate -v vs --version

mod bins;
mod config;
mod logger;
mod cli;
mod json;

use config::*;

use lib::*;
use lib::error::*;
use lib::files::{Paste, UploadFile};
use lib::range::BidirectionalRange;

use clap::ArgMatches;
use flate2::read::GzDecoder;

use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::io::{Read, Write};
use std::io::Result as IoResult;
use std::error::Error;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use log::LogLevel;

use url::Url;

macro_rules! report_error_using {
  ($using: ident, $fmt: expr, $e: expr $(, $args: expr),*) => {{
    $using!($fmt, $e, $($args)*);
    for error in error_parents($e) {
      $using!("{}", error);
    }
  }}
}

macro_rules! _report_error {
  ($fmt: expr, $e: expr $(, $args: expr),*) => (report_error_using!(error, $fmt, $e $(, $args)*))
}

macro_rules! report_error {
  ($json: expr, $fmt: expr, $e: expr $(, $args: expr),*) => {{
    if $json {
      let err = json::Error::new($e.to_string(), error_parents($e).into_iter().map(|e| e.to_string()).collect());
      let err_str = serde_json::to_string(&err).unwrap();
      println!("{}", err_str);
    } else {
      _report_error!($fmt, $e $(, $args)*);
    }
  }}
}

include!(concat!(env!("OUT_DIR"), "/version_info.rs"));

fn main() {
  std::process::exit(inner());
}

fn inner() -> i32 {
  let config = match get_config() {
    Ok(c) => c,
    Err(e) => {
      report_error_using!(println, "could not create or load bins config file: {}", &e);
      return 1;
    }
  };

  let matches = cli::create_app(config.defaults.bin.is_some()).get_matches();

  let level = if matches.is_present("debug") {
    LogLevel::Debug
  } else {
    LogLevel::Info
  };
  if let Err(e) = logger::Logger::new(level).init() {
    report_error_using!(println, "could not initialize logger: {}", &e);
    return 1;
  }

  if matches.is_present("version") {
    print_version();
    return 0;
  }

  let mut cli_options = CommandLineOptions::default();

  if matches.is_present("public") {
    cli_options.private = Some(false);
  } else if matches.is_present("private") {
    cli_options.private = Some(true);
  }

  if matches.is_present("authed") {
    cli_options.authed = Some(true);
  } else if matches.is_present("anonymous") {
    cli_options.authed = Some(false);
  }

  if matches.is_present("json") {
    cli_options.json = Some(true);
  }

  if matches.is_present("force") {
    cli_options.force = Some(true);
  }

  if matches.is_present("list-all") {
    cli_options.list_all = Some(true);
  }

  if let Some(range) = matches.value_of("range") {
    let ranges: Result<Vec<BidirectionalRange<usize>>> = range.split(',').map(|x| BidirectionalRange::<usize>::parse_usize(x)).collect();
    match ranges {
      Ok(r) => cli_options.range = Some(r),
      Err(e) => {
        report_error!(cli_options.json(), "error parsing range: {}", &e);
        return 1;
      }
    }
  }

  if let Some(name) = matches.value_of("name") {
    cli_options.name = Some(name.to_owned());
  }

  if matches.is_present("raw-urls") {
    cli_options.url_output = Some(UrlOutputMode::Raw);
  } else if matches.is_present("html-urls") {
    cli_options.url_output = Some(UrlOutputMode::Html);
  }

  #[cfg(feature = "clipboard_support")]
  {
    if matches.is_present("copy") {
      cli_options.copy = Some(true);
    } else if matches.is_present("no-copy") {
      cli_options.copy = Some(false);
    }
  }

  let config = Arc::new(config);
  let cli_options = Arc::new(cli_options);

  let bins: BTreeMap<String, Box<Bin>> = {
    let bins: Vec<Box<Bin>> = vec![
      box bins::Sprunge::new(),
      box bins::Hastebin::new(),
      box bins::Fedora::new(),
      box bins::Gist::new(config.clone(), cli_options.clone()),
      box bins::Bitbucket::new(config.clone(), cli_options.clone()),
      box bins::Pastebin::new(config.clone(), cli_options.clone())
    ];
    bins.into_iter().map(|b| (b.name().to_owned(), b)).collect()
  };

  let b = Bins {
    bins: bins,
    config: config,
    cli_options: cli_options,
    matches: matches
  };

  match b.main() {
    Ok(s) => {
      #[cfg(feature = "clipboard_support")]
      copy(&b, &s);
      println!("{}", s);
      0
    },
    Err(e) => {
      report_error!(b.cli_options.json(), "error: {}", &e);
      1
    }
  }
}

#[cfg(feature = "clipboard_support")]
fn copy(bins: &Bins, string: &str) {
  if let Some(true) = bins.cli_options.copy.or(bins.config.defaults.copy) {
    use clipboard::{ClipboardContext, ClipboardProvider};

    let mut ctx = match ClipboardContext::new() {
      Ok(c) => c,
      Err(e) => {
        report_error!(bins.cli_options.json(), "error while opening the clipboard: {}", &*e);
        return;
      }
    };

    if let Err(e) = ctx.set_contents(string.to_owned()) {
      report_error!(bins.cli_options.json(), "error while copying output to the clipboard: {}", &*e);
    }
  }
}

fn get_feature_info() -> Option<String> {
  let mut features = Vec::new();
  if cfg!(feature = "file_type_checking") {
    features.push("file_type_checking");
  }
  if cfg!(feature = "clipboard_support") {
    features.push("clipboard_support");
  }
  if features.is_empty() {
    None
  } else {
    Some(features.join(", "))
  }
}

fn print_version() {
  let name = crate_name!();
  let version = crate_version!();
  let version_info = VersionInfo::get();
  let feature_info = match get_feature_info() {
    Some(f) => format!("\nfeatures: {}", f),
    None => String::new()
  };
  let git_string = match version_info.git {
    Some(g) => format!("\ngit: {}", g),
    None => String::new()
  };
  println!("{} {}\n\ncompiled: {}\nprofile: {}{}{}",
    name,
    version,
    version_info.date,
    version_info.profile,
    git_string,
    feature_info);
}

struct Bins<'a> {
  bins: BTreeMap<String, Box<Bin>>,
  config: Arc<Config>,
  cli_options: Arc<CommandLineOptions>,
  matches: ArgMatches<'a>
}

impl<'a> Bins<'a> {
  fn main(&self) -> Result<String> {
    if self.matches.is_present("list-bins") {
      return self.list_bins();
    }
    let inputs = self.raw_inputs();
    if let Some(ref is) = inputs {
      if !is.is_empty() {
        let url: Result<Url> = Url::parse(is[0]).map_err(BinsError::UrlParse);
        if let Ok(u) = url {
          return self.download(u, if is.len() > 1 { Some(&is[1..]) } else { None }); // FIXME
        }
      }
    }
    if self.cli_options.range.is_some() {
      return Err(BinsError::Main(MainError::RangeWithUpload));
    }
    self.upload(inputs)
  }

  fn file_size_limit(&self) -> Result<Option<u64>> {
    let s = match self.config.general.file_size_limit {
      Some(ref x) => x,
      None => return Ok(None)
    };
    let mut size: Vec<char> = Vec::new();
    let mut unit: Vec<char> = Vec::new();
    for c in s.trim().chars() {
      if "0123456789.".contains(c) {
        if !unit.is_empty() {
          return Err(BinsError::Main(MainError::InvalidSizeLimit));
        }
        size.push(c);
      } else if "bBkKmMgGiI".contains(c) {
        unit.push(c);
      }
    }
    let size: f64 = size.into_iter().collect::<String>().parse().map_err(|_| BinsError::Main(MainError::InvalidSizeLimit))?;
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
        _ => return Err(BinsError::Main(MainError::InvalidSizeLimit))
      }
    };
    Ok(Some((size * unit as f64).round() as u64))
  }

  fn raw_inputs(&self) -> Option<Vec<&str>> {
    self.matches.values_of("inputs").map(|x| x.collect())
  }

  fn list_bins(&self) -> Result<String> {
    if let Some(true) = self.cli_options.json {
      let names: Vec<&String> = self.bins.keys().collect();
      serde_json::to_string(&names).map_err(BinsError::Json)
    } else {
      Ok(self.bins.keys().map(|s| s.clone()).collect::<Vec<_>>().join("\n"))
    }
  }

  fn cli_features(&self) -> HashMap<BinFeature, Option<bool>> {
    let mut map = HashMap::new();
    map.insert(BinFeature::Private, self.cli_options.private);
    map.insert(BinFeature::Public, self.cli_options.private.map(|x| !x));
    map.insert(BinFeature::Authed, self.cli_options.authed);
    map.insert(BinFeature::Anonymous, self.cli_options.authed.map(|x| !x));
    map.insert(BinFeature::SingleNaming, self.cli_options.name.as_ref().map(|_| true));
    map
  }

  fn bin_name(&self) -> Result<String> {
    self.matches.value_of("bin")
      .map(|x| x.to_owned())
      .or_else(|| self.config.defaults.bin.clone())
      .ok_or_else(|| BinsError::Main(MainError::NoBinSpecified))
  }

  fn bin(&self) -> Result<&Box<Bin>> {
    let name = self.bin_name()?;
    self.bins.get(&name).ok_or_else(|| BinsError::Main(MainError::NoSuchBin(name)))
  }

  fn check_features(&self, bin: &Box<Bin>) -> Result<()> {
    let bin_features = bin.features();
    let features = self.cli_features();
    for (feature, status) in features {
      if let Some(true) = status {
        if !bin_features.contains(&feature) {
          if let Some(true) = self.config.safety.warn_on_unsupported {
            warn!("{} does not support {} pastes", bin.name(), feature);
          }
          if let Some(true) = self.config.safety.cancel_on_unsupported {
            return match self.cli_options.force {
              Some(true) => {
                warn!("forcing upload with unsupported features");
                Ok(())
              },
              _ => Err(BinsError::Main(MainError::UnsupportedFeature(bin.name().to_owned(), feature)))
            }
          }
        }
      }
    }
    Ok(())
  }

  fn check_limit(&self, files: &Vec<(&str, File)>) -> Result<()> {
    let limit = match self.file_size_limit()? {
      Some(l) => l,
      None => return Ok(())
    };

    for &(ref name, ref file) in files {
      let metadata = file.metadata().map_err(BinsError::Io)?;
      let size = metadata.len();
      if size > limit {
        if let Some(true) = self.cli_options.force {
          warn!("{} is {} bytes, which is over the {} byte limit", name, size, limit);
        } else {
          return Err(BinsError::Main(MainError::FileOverSizeLimit {
            name: name.to_string(),
            size: size,
            limit: limit
          }));
        }
      }
    }
    Ok(())
  }

  fn get_upload_files(&self, inputs: Vec<&str>) -> Result<Vec<UploadFile>> {
    let files: Option<Vec<(&str, File)>> = inputs.into_iter()
      .map(|f| File::open(f).map(|x| Path::new(f).file_name().and_then(|f| f.to_str()).map(|of| (of, x))))
      .collect::<IoResult<_>>()
      .map_err(BinsError::Io)?;
    let files = match files {
      Some(f) => f,
      None => {
        // FIXME: json output
        error!("one or more inputs did not have a file name or did not have a valid utf-8 file name");
        return Err(BinsError::Other);
      }
    };
    self.check_limit(&files)?;
    let contents: Vec<(&str, String)> = files.into_iter()
      .map(|(n, mut f)| {
        let mut c = String::new();
        f.read_to_string(&mut c).map(|_| (n, c))
      })
      .collect::<IoResult<_>>()
      .map_err(BinsError::Io)?;
    Ok(contents.into_iter().map(|(n, c)| UploadFile::new(n.to_owned(), c)).collect())
  }

  fn inputs(&self, inputs: Option<Vec<&str>>) -> Result<Vec<UploadFile>> {
    let mut processed = match inputs {
      Some(v) => self.get_upload_files(v),
      None => {
        if let Some(message) = self.matches.value_of("message") {
          Ok(vec![UploadFile::new(String::from("message"), message.to_owned())])
        } else {
          get_stdin().map(|x| vec![x])
        }
      }
    }?;
    if let Some(ref name) = self.cli_options.name {
      if processed.len() == 1 {
        processed[0].name = name.clone();
      } else {
        return Err(BinsError::Main(MainError::NameWithMultipleFiles));
      }
    }
    Ok(processed)
  }

  fn url_output(&self, bin: &Box<Bin>, urls: &[PasteUrl]) -> Result<String> {
    let mut strings = Vec::new();
    for u in urls {
      let id = bin.id_from_html_url(u.url()).ok_or_else(|| BinsError::Main(MainError::ParseId))?;
      let raw_urls = match bin.format_raw_url(&id) {
        Some(u) => vec![u],
        None => {
          let raw_url = bin.create_raw_url(&id)?;
          raw_url.into_iter().map(|x| x.url().to_owned()).collect()
        }
      };
      for raw_url in raw_urls {
        strings.push(raw_url);
      }
    }
    Ok(strings.join("\n"))
  }

  fn upload(&self, inputs: Option<Vec<&str>>) -> Result<String> {
    let bin = self.bin()?;
    self.check_features(bin)?;

    let upload_files = self.inputs(inputs)?;
    #[cfg(feature = "file_type_checking")]
    self.check_file_types(&upload_files)?;
    let urls = bin.upload(&upload_files, self.cli_options.url_output.is_none())?;
    if let Some(UrlOutputMode::Raw) = self.cli_options.url_output {
      return self.url_output(bin, &urls);
    }
    Ok(urls.into_iter().map(|u| u.url().to_string()).collect::<Vec<String>>().join("\n"))
  }

  #[cfg(feature = "file_type_checking")]
  fn check_file_types(&self, files: &[UploadFile]) -> Result<()> {
    use magic::{Cookie, flags};

    let cookie = Cookie::open(flags::NONE).map_err(BinsError::Magic)?;
    cookie.load(&[""; 0]).map_err(BinsError::Magic)?;
    for upload_file in files {
      let kind = cookie.buffer(upload_file.content.as_bytes()).map_err(BinsError::Magic)?;
      if let Some(ref disallowed) = self.config.safety.disallowed_file_types {
        if disallowed.contains(&kind) {
          return match self.cli_options.force {
            Some(true) => {
              warn!("forcing upload with disallowed file type: ({} is {}, which is disallowed)", upload_file.name, kind);
              Ok(())
            },
            _ => Err(BinsError::InvalidFileType {
              name: upload_file.name.clone(),
              kind: kind
            })
          }
        }
      }
    }
    Ok(())
  }

  fn download(&self, url: Url, names: Option<&[&str]>) -> Result<String> {
    if names.is_some() && self.cli_options.range.is_some() {
      return Err(BinsError::Main(MainError::RangeWithNames));
    }
    let host = url.host_str().ok_or_else(|| BinsError::Main(MainError::MissingHost))?;
    let (is_html_url, bin) = match self.bins.iter().find(|&(_, b)| b.raw_host() == host) {
      Some(b) => (false, b.1),
      None => {
        match self.bins.iter().find(|&(_, b)| b.html_host() == host) {
          Some(b) => (true, b.1),
          None => return Err(BinsError::Main(MainError::NoSuchHost(host.to_owned())))
        }
      }
    };
    let id = if is_html_url {
      bin.id_from_html_url(url.as_str())
    } else {
      bin.id_from_raw_url(url.as_str())
    };
    let id = id.ok_or_else(|| BinsError::Main(MainError::ParseId))?;
    if let Some(ref output_mode) = self.cli_options.url_output {
      let urls = match *output_mode {
        UrlOutputMode::Html => bin.create_html_url(&id),
        UrlOutputMode::Raw => bin.create_raw_url(&id)
      }?;
      return Ok(urls.into_iter().map(|u| u.url().to_string()).collect::<Vec<_>>().join("\n"));
    }
    if let Some(true) = self.cli_options.list_all {
      let urls = bin.create_raw_url(&id)?;
      return Ok(urls.into_iter()
        .map(|u| u.name()
          .map(|p| p.name())
          .unwrap_or_else(|| String::from("<unknown>")))
        .collect::<Vec<_>>()
        .join("\n"));
    }
    let download_info = if let Some(ref range) = self.cli_options.range {
      DownloadInfo::range(range)
    } else if let Some(ns) = names {
      DownloadInfo::names(ns)
    } else {
      DownloadInfo::empty()
    };
    let download = bin.download(&id, &download_info)?;
    if let Some(true) = self.cli_options.json {
      let j = serde_json::to_string(&download).map_err(BinsError::Json)?;
      Ok(j)
    } else {
      let output = match download {
        Paste::Single(f) => f.content,
        Paste::Multiple(fs) =>
          fs.iter()
            .map(|f| format!("==> {} <==\n\n{}", f.name.name(), f.content))
            .collect::<Vec<_>>()
            .join("\n")
      };
      Ok(output)
    }
  }
}

fn get_stdin() -> Result<UploadFile> {
  let mut content = String::new();
  let mut stdin = std::io::stdin();
  stdin.read_to_string(&mut content).map_err(BinsError::Io)?;
  Ok(UploadFile::new("stdin".to_owned(), content))
}

fn error_parents(error: &Error) -> Vec<&Error> {
  let mut parents = Vec::new();
  let mut last_error = error;
  loop {
    match last_error.cause() {
      None => break,
      Some(e) => {
        parents.push(e);
        last_error = e;
      }
    }
  }
  parents
}

fn get_config() -> Result<Config> {
  let mut f = match find_config_path() {
    Some(p) => File::open(p).map_err(BinsError::Io)?,
    None => create_config_file()?
  };
  let mut content = String::new();
  f.read_to_string(&mut content).map_err(BinsError::Io)?;
  toml::from_str(&content).map_err(BinsError::Toml)
}

fn create_xdg_config_file() -> Result<File> {
  if let Ok(xdg_dir) = std::env::var("XDG_CONFIG_DIR") {
    let xdg_path = Path::new(&xdg_dir);
    let xdg_config_path = xdg_path.join("bins.cfg");
    if xdg_path.exists() && xdg_path.is_dir() && !xdg_config_path.exists() {
      return OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(xdg_config_path)
        .map_err(BinsError::Io);
    }
  }
  Err(BinsError::Config)
}

fn create_home_config_file() -> Result<File> {
  if let Ok(home_dir) = std::env::var("HOME") {
    let home = Path::new(&home_dir);
    let home_folder = home.join(".config");
    let home_folder_config = home_folder.join("bins.cfg");
    if home_folder.exists() && home_folder.is_dir() && !home_folder_config.exists() {
      return OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(home_folder_config)
        .map_err(BinsError::Io);
    }
    let home_config = Path::new(&home_dir).join(".bins.cfg");
    if home.exists() && home.is_dir() && !home_config.exists() {
      return OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(home_config)
        .map_err(BinsError::Io);
    }
  }
  Err(BinsError::Config)
}

fn create_config_file() -> Result<File> {
  let mut f = match create_xdg_config_file() {
    Ok(f) => f,
    Err(_) => match create_home_config_file() {
      Ok(hf) => hf,
      Err(_) => return Err(BinsError::Config)
    }
  };
  let mut default_config = String::new();
  GzDecoder::new(config::DEFAULT_CONFIG_GZIP)
    .map_err(BinsError::Io)?
    .read_to_string(&mut default_config)
    .map_err(BinsError::Io)?;
  f.write_all(default_config.as_bytes()).map_err(BinsError::Io)?;
  f.seek(SeekFrom::Start(0)).map_err(BinsError::Io)?;
  Ok(f)
}

fn find_config_path() -> Option<PathBuf> {
  if let Ok(xdg_dir) = std::env::var("XDG_CONFIG_DIR") {
    let xdg_config_path = Path::new(&xdg_dir).join("bins.cfg");
    if xdg_config_path.exists() {
      return Some(xdg_config_path.to_owned());
    }
  }
  if let Ok(home_dir) = std::env::var("HOME") {
    let home_config_folder = Path::new(&home_dir).join(".config").join("bins.cfg");
    if home_config_folder.exists() {
      return Some(home_config_folder.to_owned());
    }
    let home_config = Path::new(&home_dir).join(".bins.cfg");
    if home_config.exists() {
      return Some(home_config.to_owned());
    }
  }
  None
}
