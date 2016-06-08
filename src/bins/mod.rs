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
use std::collections::HashMap;
use config::types::Config;
use bins::error::*;
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

  pub fn get_engine(&self) -> Result<Box<Engine>> {
    let service = match self.arguments.service {
      Some(ref s) => s,
      None => return Err("no service was specified and no default service was set.".into())
    };
    match service.to_lowercase().as_ref() {
      "gist" => Ok(Box::new(Gist::new())),
      "hastebin" => Ok(Box::new(Hastebin::new())),
      "pastie" => Ok(Box::new(Pastie::new())),
      "pastebin" => Ok(Box::new(Pastebin::new())),
      _ => Err(format!("unknown service \"{}\"", service).into())
    }
  }

  fn read_file<P: AsRef<Path>>(&self, p: P) -> Result<String> {
    let path = p.as_ref();
    let name = match path.to_str() {
      Some(s) => s,
      None => return Err(String::from("file name was not valid unicode").into())
    };
    if !path.exists() {
      return Err(format!("{} does not exist", name).into());
    }
    if !path.is_file() {
      return Err(format!("{} is not a file", name).into());
    }
    let mut file = match File::open(path) {
      Ok(f) => f,
      Err(e) => {
        return Err(format!("could not open {}: {}", name, e).into());
      }
    };
    let mut s = String::new();
    if let Err(e) = file.read_to_string(&mut s) {
      return Err(format!("could not read {}: {}", name, e).into());
    }
    Ok(s)
  }

  fn read_file_to_pastefile<P: AsRef<Path>>(&self, p: P) -> Result<PasteFile> {
    let path = p.as_ref();
    match self.read_file(path) {
      Ok(s) => {
        let n = match path.file_name() {
          Some(x) => x,
          None => return Err("not a valid file name".into())
        };
        Ok(PasteFile::new(n.to_string_lossy().into_owned(), s))
      },
      Err(s) => return Err(s)
    }
  }

  pub fn get_to_paste(&self) -> Result<Vec<PasteFile>> {
    let arguments = &self.arguments;
    let message = &arguments.message;
    let paste_files: Vec<PasteFile> = if !message.is_empty() {
      vec![PasteFile::new(String::from("message"), message.to_owned())]
    } else if !arguments.files.is_empty() {
      let files = arguments.files.clone();
      let results = files.iter()
        .map(|s| Path::new(s))
        .map(|p| self.read_file_to_pastefile(p))
        .map(|r| r.map_err(|e| e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n")))
        .collect::<Vec<_>>();
      for res in results.iter().cloned() {
        if res.is_err() {
          return Err(res.err().unwrap().into());
        }
      }
      let mut pastes = results.iter().cloned().map(|r| r.unwrap()).collect::<Vec<_>>();
      self.handle_duplicate_file_names(&mut pastes);
      pastes
    } else {
      let mut buffer = String::new();
      if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        return Err(format!("error reading stdin: {}", e).into());
      }
      vec![PasteFile::new(String::from("stdin"), buffer)]
    };
    Ok(paste_files)
  }

  fn handle_duplicate_file_names(&self, pastes: &mut Vec<PasteFile>) {
    let clone = pastes.clone();
    let names = clone.iter().map(|p| &p.name).collect::<Vec<_>>();
    let mut names_map: HashMap<String, i32> = HashMap::new();
    for mut paste in pastes {
      let name = paste.name.clone();
      if names.contains(&&name) {
        let number = names_map.entry(name.clone()).or_insert(1);
        paste.name = format!("{}_{}", name, number);
        *number += 1;
      }
    }
  }
}
