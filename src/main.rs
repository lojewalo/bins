extern crate bins as lib;
extern crate hyper;
#[cfg(feature = "openssl")]
extern crate hyper_openssl;
#[cfg(feature = "rustls")]
extern crate hyper_rustls;
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
#[macro_use]
extern crate error_chain;
extern crate uuid;

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

type Client = hyper::Client<hyper::client::connect::HttpConnector>;

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

  let matches = cli::create_app().get_matches();

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

  if matches.is_present("list-bins") && matches.is_present("bin") {
    error!("--bin cannot be used with --list-bins");
    return 1;
  }

  if !matches.is_present("list-bins") && !config.defaults.bin.is_some() && !matches.is_present("bin") {
    error!("you must specify a bin with --bin or set a default bin");
    return 1;
  }

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

  if let Some(path) = matches.value_of("output") {
    cli_options.output = Some(path.to_owned());
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

  let bins: BTreeMap<String, Box<dyn Bin>> = {
    let bins: Vec<Box<dyn Bin>> = vec![
      Box::new(bins::Sprunge::new()),
      Box::new(bins::Hastebin::new()),
      Box::new(bins::Fedora::new()),
      Box::new(bins::Gist::new(config.clone(), cli_options.clone())),
      Box::new(bins::Bitbucket::new(config.clone(), cli_options.clone())),
      Box::new(bins::Pastebin::new(config.clone(), cli_options.clone())),
      Box::new(bins::PasteGg::new(config.clone(), cli_options.clone())),
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
  if cfg!(feature = "openssl") {
    features.push("openssl");
  }
  if cfg!(feature = "rustls") {
    features.push("rustls");
  }
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
  bins: BTreeMap<String, Box<dyn Bin>>,
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
        if let Ok(u) = Url::parse(is[0]) {
          return self.download(u, if is.len() > 1 { Some(&is[1..]) } else { None }); // FIXME
        }
      }
    }
    if self.cli_options.range.is_some() {
      bail!("cannot upload with --range");
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
          bail!("the file size limit specified in the config is invalid");
        }
        size.push(c);
      } else if "bBkKmMgGiI".contains(c) {
        unit.push(c);
      }
    }
    let size: f64 = size.into_iter().collect::<String>().parse().chain_err(|| "the file size limit specified in the config is invalid")?;
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
        _ => bail!("the file size limit specified in the config is invalid")
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
      serde_json::to_string(&names).chain_err(|| "could not serialize list of bins")
    } else {
      Ok(self.bins.keys().cloned().collect::<Vec<_>>().join("\n"))
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
      .and_then(|x| if x.trim().is_empty() { None } else { Some(x) })
      .ok_or_else(|| "no bin was specified".into())
  }

  fn bin(&self) -> Result<&Box<dyn Bin>> {
    let name = self.bin_name()?;
    self.bins.get(&name).ok_or_else(|| format!("there is no bin called \"{}\"", name).into())
  }

  fn check_features(&self, bin: &dyn Bin) -> Result<()> {
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
              _ => bail!("bins stopped because {} does not support {} pastes", bin.name(), feature)
            }
          }
        }
      }
    }
    Ok(())
  }

  fn check_limit(&self, files: &[(&str, File)]) -> Result<()> {
    let limit = match self.file_size_limit()? {
      Some(l) => l,
      None => return Ok(())
    };

    for &(name, ref file) in files {
      let metadata = file.metadata()?;
      let size = metadata.len();
      if size > limit {
        if let Some(true) = self.cli_options.force {
          warn!("{} is {} bytes, which is over the {} byte limit", name, size, limit);
        } else {
          bail!("{} is {} byte{}, which is over the size limit of {} byte{}",
            name,
            size,
            if size == 1 { "" } else { "s" },
            limit,
            if limit == 1 { "" } else { "s" });
        }
      }
    }
    Ok(())
  }

  fn get_upload_files(&self, inputs: Vec<&str>) -> Result<Vec<UploadFile>> {
    let files: Option<Vec<(&str, File)>> = inputs.into_iter()
      .map(|f| File::open(f).map(|x| Path::new(f).file_name().and_then(|f| f.to_str()).map(|of| (of, x))))
      .collect::<IoResult<_>>()?;
    let files = match files {
      Some(f) => f,
      None => {
        error!("one or more inputs did not have a file name or did not have a valid utf-8 file name");
        bail!("invalid utf-8 file names");
      }
    };
    self.check_limit(&files)?;
    let contents: Vec<(&str, String)> = files.into_iter()
      .map(|(n, mut f)| {
        let mut c = String::new();
        f.read_to_string(&mut c).map(|_| (n, c))
      })
      .collect::<IoResult<_>>()?;
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
        bail!("cannot use --name with multiple upload files");
      }
    }
    Ok(processed)
  }

  fn url_output(&self, bin: &dyn Bin, urls: &[PasteUrl]) -> Result<String> {
    let mut strings = Vec::new();
    for u in urls {
      let id = bin.id_from_html_url(u.url()).ok_or_else(|| ErrorKind::Msg("could not parse ID from URL".into()))?;
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
    self.check_features(bin.as_ref())?;

    let upload_files = self.inputs(inputs)?;
    #[cfg(feature = "file_type_checking")]
    self.check_file_types(&upload_files)?;
    let urls = bin.upload(&upload_files, self.cli_options.url_output.is_none())?;
    if let Some(UrlOutputMode::Raw) = self.cli_options.url_output {
      return self.url_output(bin.as_ref(), &urls);
    }
    Ok(urls.into_iter().map(|u| u.url().to_string()).collect::<Vec<String>>().join("\n"))
  }

  #[cfg(feature = "file_type_checking")]
  fn check_file_types(&self, files: &[UploadFile]) -> Result<()> {
    use magic::{Cookie, flags};

    let cookie = Cookie::open(flags::NONE)?;
    cookie.load(&[""; 0])?;
    for upload_file in files {
      let kind = cookie.buffer(upload_file.content.as_bytes())?;
      if let Some(ref disallowed) = self.config.safety.disallowed_file_types {
        if disallowed.contains(&kind) {
          return match self.cli_options.force {
            Some(true) => {
              warn!("forcing upload with disallowed file type: ({} is {}, which is disallowed)", upload_file.name, kind);
              Ok(())
            },
            _ => Err(ErrorKind::InvalidFileType(upload_file.name.clone(), kind).into())
          }
        }
      }
    }
    Ok(())
  }

  fn download(&self, url: Url, names: Option<&[&str]>) -> Result<String> {
    if names.is_some() && self.cli_options.range.is_some() {
      bail!("cannot specify file names with --range");
    }
    let host = url.host_str().ok_or_else(|| ErrorKind::Msg("url was missing a host".into()))?;
    let (is_html_url, bin) = match self.bins.iter().find(|&(_, b)| b.raw_host() == host) {
      Some(b) => (false, b.1),
      None => {
        match self.bins.iter().find(|&(_, b)| b.html_host() == host) {
          Some(b) => (true, b.1),
          None => bail!("no bin uses the hostname {}", host)
        }
      }
    };
    let id = if is_html_url {
      bin.id_from_html_url(url.as_str())
    } else {
      bin.id_from_raw_url(url.as_str())
    };
    let id = id.ok_or_else(|| ErrorKind::Msg("could not parse ID from URL".into()))?;
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
    if let Some(ref path_str) = self.cli_options.output {
      let path = Path::new(path_str);
      if !path.exists() {
        bail!("{} does not exist", path_str);
      }
      if !path.is_dir() {
        bail!("{} is not a directory", path_str);
      }
      let downloads = match download {
        Paste::Single(f) => vec![f],
        Paste::Multiple(fs) => fs
      };
      for download in downloads {
        let download_name = download.name.name();
        let mut download_path = path.join(&download_name);
        let mut tries = 0;
        while download_path.exists() {
          tries += 1;
          let mut parts: Vec<String> = download_name.split('.').map(|x| x.to_string()).collect();
          let len = parts.len();
          let index = match len {
            1 => 0,
            _ => len - 2
          };
          parts[index] = format!("{}_{}", parts[index], tries);
          download_path = path.join(parts.join("."));
        }
        let name = download_path.to_string_lossy().into_owned();
        let mut file = OpenOptions::new()
          .write(true)
          .create(true)
          .open(download_path)
          .chain_err(|| format!("could not open {}", name))?;
        file
          .write_all(download.content.as_bytes())
          .chain_err(|| format!("could not write to {}", name))?;
      }
      return Ok(Default::default());
    }
    if let Some(true) = self.cli_options.json {
      let j = serde_json::to_string(&download)?;
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
  stdin.read_to_string(&mut content)?;
  Ok(UploadFile::new("stdin".to_owned(), content))
}

fn error_parents(error: &dyn Error) -> Vec<&dyn Error> {
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
    Some(p) => File::open(p)?,
    None => create_config_file()?
  };
  let mut content = String::new();
  f.read_to_string(&mut content)?;
  toml::from_str(&content).chain_err(|| "could not parse configuration file")
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
        .chain_err(|| "could not open/create XDG_CONFIG_DIR config file");
    }
  }
  Err(ErrorKind::Config.into())
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
        .chain_err(|| "could not open/create HOME/.config config file");
    }
    let home_config = Path::new(&home_dir).join(".bins.cfg");
    if home.exists() && home.is_dir() && !home_config.exists() {
      return OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(home_config)
        .chain_err(|| "could not open/create HOME config file");
    }
  }
  Err(ErrorKind::Config.into())
}

fn create_config_file() -> Result<File> {
  let mut f = match create_xdg_config_file() {
    Ok(f) => f,
    Err(_) => match create_home_config_file() {
      Ok(hf) => hf,
      Err(_) => return Err(ErrorKind::Config.into())
    }
  };
  let mut default_config = String::new();
  GzDecoder::new(config::DEFAULT_CONFIG_GZIP)?
    .read_to_string(&mut default_config)?;
  f.write_all(default_config.as_bytes())?;
  f.seek(SeekFrom::Start(0))?;
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

#[cfg(feature = "openssl")]
pub fn new_client() -> Client {
  use hyper_openssl::OpensslClient;
  Client::with_connector(HttpsConnector::new(OpensslClient::new().unwrap()))
}

#[cfg(feature = "rustls")]
pub fn new_client() -> Client {
  use hyper_rustls::TlsClient;
  Client::with_connector(HttpsConnector::new(TlsClient::new()))
}
