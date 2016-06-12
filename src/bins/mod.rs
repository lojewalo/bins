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
use bins::engines::sprunge::Sprunge;
use hyper::Url;

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
      "sprunge" => Ok(Box::new(Sprunge::new())),
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
      Err(s) => Err(s)
    }
  }

  pub fn get_to_paste(&self) -> Result<Vec<PasteFile>> {
    let arguments = &self.arguments;
    let message = &arguments.message;
    let paste_files: Vec<PasteFile> = if message.is_some() {
      vec![PasteFile::new(String::from("message"), message.clone().unwrap())]
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
      let mut pastes = results.iter().cloned().map(|r| r.unwrap()).filter(|p| !p.data.trim().is_empty()).collect::<Vec<_>>();
      self.handle_duplicate_file_names(&mut pastes);
      pastes
    } else {
      let mut buffer = String::new();
      if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        return Err(format!("error reading stdin: {}", e).into());
      }
      vec![PasteFile::new(String::from("stdin"), buffer)]
    };
    if paste_files.iter().filter(|p| !p.data.trim().is_empty()).count() < 1 {
      return Err("no files (or all empty files) to paste".into());
    }
    Ok(paste_files)
  }

  fn handle_duplicate_file_names(&self, pastes: &mut Vec<PasteFile>) {
    let mut names_map: HashMap<String, i32> = HashMap::new();
    for mut paste in pastes {
      let name = paste.name.clone();
      if names_map.contains_key(&name) {
        let parts = name.rsplit('.');
        let (beginning, end) = if parts.clone().count() > 1 {
          let mut beginning_parts = parts.clone().skip(1).collect::<Vec<_>>();
          beginning_parts.reverse();
          let beginning = beginning_parts.join(".");
          let end = parts.take(1).next().map_or(String::new(), |s| String::from(".") + s);
          (beginning, end)
        } else {
          (name.clone(), String::from(""))
        };
        let number = names_map.entry(name.clone()).or_insert(1);
        paste.name = format!("{}_{}{}", beginning, number, end);
        *number += 1;
      }
      names_map.entry(name.clone()).or_insert(1);
    }
  }

  fn get_engine_for_url(&self, url: &Url) -> Result<Box<Engine>> {
    let domain = match url.domain() {
      Some(d) => d,
      None => return Err("input url had no domain".into())
    };
    let engine: Box<Engine> = match domain {
      "gist.github.com" => Box::new(Gist::new()),
      "hastebin.com" => Box::new(Hastebin::new()),
      "pastebin.com" => Box::new(Pastebin::new()),
      "pastie.org" => Box::new(Pastie::new()),
      "sprunge.us" => Box::new(Sprunge::new()),
      _ => return Err(format!("could not find a bin for domain {}", domain).into())
    };
    Ok(engine)
  }

  fn get_raw(&self, url_string: &str) -> Result<String> {
    // can't use try!() because url::parser is private, and ParseError is at url::parser::ParseError
    let mut url = match Url::parse(url_string.as_ref()) {
      Ok(u) => u,
      Err(e) => return Err(e.to_string().into())
    };
    let engine = try!(self.get_engine_for_url(&url));
    engine.get_raw(self, &mut url)
  }

  pub fn get_output(&self) -> Result<String> {
    if let Some(ref input) = self.arguments.input {
      return self.get_raw(input);
    }
    let to_paste = try!(self.get_to_paste());
    let engine = try!(self.get_engine());
    engine.upload(self, &to_paste)
  }
}
