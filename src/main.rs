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

macro_rules! option {
  ($e: expr) => {{
    match $e {
      Some(x) => x,
      None => return None
    }
  }}
}

mod bins;
mod config;
mod logger;

use config::*;

use lib::*;
use lib::error::*;
use lib::files::{Paste, UploadFile};

use clap::{App, Arg, ArgMatches};
use flate2::read::GzDecoder;

use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::io::{Read, Write};
use std::io::Result as IoResult;
use std::error::Error;
use std::collections::HashMap;
use std::sync::Arc;

use log::LogLevel;

use url::Url;

fn main() {
  std::process::exit(inner());
}

fn inner() -> i32 {
  let config = match get_config() {
    Ok(c) => c,
    Err(e) => {
      println!("could not create or load bins config file: {}", e);
      for error in error_parents(&e) {
        println!("{}", error);
      }
      return 1;
    }
  };

  let matches = App::new("bins")
    .about("A tool for pasting from the terminal")
    .author(crate_authors!())
    .version(crate_version!())
    .version_message("print version information and exit")
    .version_short("v")
    .help_message("print help information and exit")
    .arg(Arg::with_name("inputs")
      .help("inputs to the program, either files or URLs")
      .takes_value(true)
      .value_name("input")
      .multiple(true))
    .arg(Arg::with_name("debug")
      .long("debug")
      .short("d")
      .help("enable debug output"))
    .arg(Arg::with_name("bin")
      .long("bin")
      .short("b")
      .help("specify the upload bin")
      .required(config.defaults.bin.is_none())
      .takes_value(true)
      .value_name("bin")
      .possible_values(&["hastebin", "sprunge", "gist"]))
    .arg(Arg::with_name("public")
      .long("public")
      .short("P")
      .help("set the paste to be public")
      .conflicts_with("private"))
    .arg(Arg::with_name("private")
      .long("private")
      .short("p")
      .help("set the paste to be private"))
    .arg(Arg::with_name("authed")
      .long("authed")
      .short("a")
      .help("set the paste to be uploaded while authenticated")
      .conflicts_with("anonymous"))
    .arg(Arg::with_name("anonymous")
      .long("anonymous")
      .short("A")
      .help("set the paste to be uploaded while not authenticated"))
    .arg(Arg::with_name("json")
      .long("json")
      .short("j")
      .help("make bins output JSON information"))
    .get_matches();

  let level = if matches.is_present("debug") {
    LogLevel::Debug
  } else {
    LogLevel::Info
  };
  if let Err(e) = logger::Logger::new(level).init() {
    println!("could not initialize logger: {}", e);
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

  let config = Arc::new(config);
  let cli_options = Arc::new(cli_options);

  let bins: HashMap<&str, Box<Bin>> = {
    let mut map: HashMap<&str, Box<Bin>> = HashMap::new();
    map.insert("sprunge", box bins::Sprunge::new(config.clone(), cli_options.clone()));
    map.insert("hastebin", box bins::Hastebin::new(config.clone(), cli_options.clone()));
    map.insert("gist", box bins::Gist::new(config.clone(), cli_options.clone()));
    map
  };

  let b = Bins {
    bins: bins,
    config: config,
    cli_options: cli_options,
    matches: matches
  };

  b.main()
}

struct Bins<'a> {
  bins: HashMap<&'static str, Box<Bin>>,
  config: Arc<Config>,
  cli_options: Arc<CommandLineOptions>,
  matches: ArgMatches<'a>
}

impl<'a> Bins<'a> {
  fn main(&self) -> i32 {
    let inputs: Option<Vec<&str>> = self.matches.values_of("inputs").map(|x| x.collect());
    if let Some(ref is) = inputs {
      if !is.is_empty() {
        let url: Result<Url> = Url::parse(&is[0]).map_err(BinsError::UrlParse);
        if let Ok(u) = url {
          return self.download(u, if is.len() > 1 { Some(&is[1..]) } else { None });
        }
      }
    }
    self.upload(inputs)
  }

  fn upload(&self, inputs: Option<Vec<&str>>) -> i32 {
    let bin_name = self.matches.value_of("bin").map(|x| x.to_owned()).or_else(|| self.config.defaults.bin.clone()).expect("no bin specified");
    let possible_bin = self.bins.get(bin_name.as_str());
    let bin = match possible_bin {
      Some(b) => b,
      None => {
        error!("there is no bin called \"{}\"", bin_name);
        return 1;
      }
    };

    let bin_features = bin.features();
    let features = {
      let mut map = HashMap::new();
      map.insert(BinFeature::Private, self.cli_options.private);
      map.insert(BinFeature::Public, self.cli_options.private.map(|x| !x));
      map.insert(BinFeature::Authed, self.cli_options.authed);
      map.insert(BinFeature::Anonymous, self.cli_options.authed.map(|x| !x));
      map
    };
    for (feature, status) in features {
      if let Some(true) = status {
        if !bin_features.contains(&feature) {
          if let Some(true) = self.config.safety.warn_on_unsupported {
            warn!("{} does not support {} pastes", bin.name(), feature);
          }
          if let Some(true) = self.config.safety.cancel_on_unsupported {
            error!("bins stopped because an unsupported feature was used with {}", bin.name());
            return 1;
          }
        }
      }
    }

    let upload_files = match inputs {
      Some(v) => get_upload_files(v),
      None => get_stdin().map(|x| vec![x])
    };
    let upload_files = match upload_files {
      Ok(u) => u,
      Err(e) => {
        error!("could not get input: {}", e);
        return 1;
      }
    };
    match bin.upload(&upload_files) {
      Err(e) => {
        error!("error uploading to {}: {}", bin.name(), e);
        for error in error_parents(&e) {
          error!("{}", error);
        }
        return 1;
      },
      Ok(u) => println!("{}", u)
    }
    0
  }

  fn download(&self, url: Url, names: Option<&[&str]>) -> i32 {
    let host = match url.host_str() {
      Some(h) => h,
      None => {
        error!("invalid url (no host): {}", url.as_str());
        return 1;
      }
    };
    let (is_html_url, bin) = match self.bins.iter().find(|&(_, b)| b.raw_host() == host) {
      Some(b) => (false, b.1),
      None => {
        match self.bins.iter().find(|&(_, b)| b.html_host() == host) {
          Some(b) => (true, b.1),
          None => {
            error!("no bin uses the hostname {}", host);
            return 1;
          }
        }
      }
    };
    let id = if is_html_url {
      bin.id_from_html_url(url.as_str())
    } else {
      bin.id_from_raw_url(url.as_str())
    };
    let id = match id {
      Some(i) => i,
      None => {
        error!("could not extract paste ID from {}", url.as_str());
        return 1;
      }
    };
    let download = match bin.download(&id, names) {
      Ok(d) => d,
      Err(e) => {
        error!("could not download ID {}: {}", id, e);
        return 1;
      }
    };
    if let Some(true) = self.cli_options.json {
      match serde_json::to_string(&download) {
        Ok(j) => println!("{}", j),
        Err(e) => {
          error!("error converting download to json: {}", e);
          return 1;
        }
      }
    } else {
      match download {
        Paste::Single(f) => {
          println!("{}", f.content);
        },
        Paste::MultiDownloaded(fs) => {
          for f in fs {
            println!("==> {} <==\n\n{}", f.name.name(), f.content);
          }
        }
      }
    }
    0
  }
}

fn get_stdin() -> Result<UploadFile> {
  let mut content = String::new();
  let mut stdin = std::io::stdin();
  stdin.read_to_string(&mut content).map_err(BinsError::Io)?;
  Ok(UploadFile::new("stdin".to_owned(), content))
}

fn get_upload_files(inputs: Vec<&str>) -> Result<Vec<UploadFile>> {
  let files: Vec<(&str, File)> = inputs.into_iter()
    .map(|f| File::open(f).map(|x| (f, x)))
    .collect::<IoResult<_>>()
    .map_err(BinsError::Io)?;
  let contents: Vec<(&str, String)> = files.into_iter()
    .map(|(n, mut f)| {
      let mut c = String::new();
      f.read_to_string(&mut c).map(|_| (n, c))
    })
    .collect::<IoResult<_>>()
    .map_err(BinsError::Io)?;
  Ok(contents.into_iter().map(|(n, c)| UploadFile::new(n.to_owned(), c)).collect())
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
