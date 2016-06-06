#[macro_use]
pub mod macros;
pub mod error;
pub mod arguments;
pub mod configuration;
pub mod engines;

extern crate std;
extern crate config;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use config::types::Config;
use bins::arguments::Arguments;
use bins::engines::Engine;
use bins::engines::gist::Gist;
use bins::engines::hastebin::Hastebin;
use bins::engines::pastie::Pastie;
use bins::engines::pastebin::Pastebin;

#[derive(Clone)]
pub struct PasteFile {
  pub name: String,
  pub data: String
}

impl PasteFile {
  fn new(name: String, data: String) -> Self {
    PasteFile { name: name, data: data }
  }
}

pub struct Bins {
  pub config: Config,
  pub arguments: Arguments
}

impl Bins {
  pub fn new(config: Config, arguments: Arguments) -> Self {
    Bins {
      config: config,
      arguments: arguments
    }
  }

  pub fn get_engine(&self) -> Result<Box<Engine>, String> {
    match self.arguments.service.to_lowercase().as_ref() {
      "gist" => Ok(Box::new(Gist::new())),
      "hastebin" => Ok(Box::new(Hastebin::new())),
      "pastie" => Ok(Box::new(Pastie::new())),
      "pastebin" => Ok(Box::new(Pastebin::new())),
      _ => Err(format!("unknown service \"{}\"", self.arguments.service))
    }
  }

  fn read_file<P: AsRef<Path>>(&self, p: P) -> Result<String, String> {
    let path = p.as_ref();
    let name = match path.to_str() {
      Some(s) => s,
      None => return Err(String::from("file name was not valid unicode"))
    };
    if !path.exists() {
      return Err(format!("{} does not exist", name));
    }
    if !path.is_file() {
      return Err(format!("{} is not a file", name));
    }
    let mut file = match File::open(path) {
      Ok(f) => f,
      Err(e) => {
        return Err(format!("could not open {}: {}", name, e));
      }
    };
    let mut s = String::new();
    if let Err(e) = file.read_to_string(&mut s) {
      return Err(format!("could not read {}: {}", name, e));
    }
    Ok(s)
  }

  fn read_file_to_pastefile<P: AsRef<Path>>(&self, p: P) -> Result<PasteFile, String> {
    let path = p.as_ref();
    match self.read_file(path) {
      Ok(s) => {
        let n = match path.file_name() {
          Some(x) => x,
          None => return Err(String::from("not a valid file name"))
        };
        Ok(PasteFile::new(n.to_string_lossy().into_owned(), s))
      },
      Err(s) => return Err(s)
    }
  }

  pub fn get_to_paste(&self) -> Result<Vec<PasteFile>, String> {
    let arguments = &self.arguments;
    let message = &arguments.message;
    let paste_files: Vec<PasteFile> = if !message.is_empty() {
      vec![PasteFile::new(String::from("message"), message.to_owned())]
    } else if !arguments.files.is_empty() {
      let files = arguments.files.clone();
      let results = files.iter()
        .map(|s| Path::new(s))
        .map(|p| self.read_file_to_pastefile(p))
        .collect::<Vec<_>>();
      for res in results.iter().cloned() {
        if res.is_err() {
          return Err(res.err().unwrap().to_string());
        }
      }
      results.iter().cloned().map(|r| r.unwrap()).collect()
    } else {
      let mut buffer = String::new();
      if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        return Err(format!("error reading stdin: {}", e));
      }
      vec![PasteFile::new(String::from("stdin"), buffer)]
    };
    Ok(paste_files)
  }
}
