#[macro_use]
pub mod macros;
pub mod arguments;
pub mod configuration;
pub mod engines;
pub mod error;
pub mod network;
#[cfg(feature = "file_type_checking")]
pub mod magic;

extern crate std;
extern crate toml;
extern crate rustc_serialize;

pub use self::engines::bitbucket::Bitbucket;
pub use self::engines::gist::Gist;
pub use self::engines::hastebin::Hastebin;
pub use self::engines::pastebin::Pastebin;
pub use self::engines::pastie::Pastie;
pub use self::engines::sprunge::Sprunge;

use bins::error::*;
use bins::arguments::Arguments;
use bins::configuration::BinsConfiguration;
use bins::engines::Bin;
use hyper::Url;
use rustc_serialize::json;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

cfg_if! {
  if #[cfg(feature = "file_type_checking")] {
    fn check_magic(bins: &Bins, pastes: &[PasteFile]) -> Result<()> {
      use bins::magic::MagicWrapper;
      let magic = try!(MagicWrapper::new(0, true));
      let disallowed_types = bins.config.get_general_disallowed_file_types();
      if let Some(disallowed_types) = disallowed_types {
        for file in pastes {
          let magic_type = try!(magic.magic_buffer(file.data.as_bytes()));
          if disallowed_types.contains(&magic_type.to_lowercase()) {
            return Err(format!("{} is a disallowed type ({}). use --force to force upload",
              file.name, magic_type).into());
          }
        }
      }
      Ok(())
    }
  } else {
    fn check_magic(_: &Bins, _: &[PasteFile]) -> Result<()> {
      Ok(())
    }
  }
}

#[derive(Debug, Clone)]
pub struct PasteFile {
  pub name: String,
  pub data: String
}

impl PasteFile {
  fn new(name: String, data: String) -> Self {
    PasteFile {
      name: name,
      data: data
    }
  }
}

pub struct Bins {
  pub config: BinsConfiguration,
  pub arguments: Arguments
}

impl Bins {
  pub fn new(config: BinsConfiguration, arguments: Arguments) -> Self {
    Bins {
      config: config,
      arguments: arguments
    }
  }

  pub fn get_engine(&self) -> Result<&Box<Bin>> {
    let service = match self.arguments.service {
      Some(ref s) => s,
      None => return Err("no service was specified and no default service was set.".into()),
    };
    match engines::get_bin_by_name(service) {
      Some(engine) => Ok(engine),
      None => Err(format!("unknown service \"{}\"", service).into()),
    }
  }

  fn read_file<P: AsRef<Path>>(&self, p: P) -> Result<String> {
    let path = p.as_ref();
    let name = some_or_err!(path.to_str(), "file name was not valid unicode".into());
    if !path.exists() {
      return Err(format!("{} does not exist", name).into());
    }
    if !path.is_file() {
      return Err(format!("{} is not a file", name).into());
    }
    let mut file = match File::open(path) {
      Ok(f) => f,
      Err(e) => return Err(format!("could not open {}: {}", name, e).into()),
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
        let n = some_or_err!(path.file_name(), "not a valid file name".into());
        Ok(PasteFile::new(n.to_string_lossy().into_owned(), s))
      }
      Err(s) => Err(s),
    }
  }

  pub fn get_to_paste(&self) -> Result<Vec<PasteFile>> {
    let arguments = &self.arguments;
    let message = &arguments.message;
    let paste_files: Vec<PasteFile> = if message.is_some() {
      let name = arguments.name.clone().unwrap_or_else(|| String::from("message"));
      vec![PasteFile::new(name, message.clone().unwrap())]
    } else if !arguments.files.is_empty() {
      let files = arguments.files.clone();
      let results = files.iter()
        .map(|s| Path::new(s))
        .map(|p| {
          if !self.arguments.force {
            let metadata = match p.metadata() {
              Ok(m) => m,
              Err(e) => return Err(e.to_string().into()),
            };
            let size = metadata.len();
            let limit = try!(self.config.get_general_file_size_limit());
            if let Some(limit) = limit {
              if size > limit {
                return Err(format!("{} ({} bytes) was larger than the upload limit ({} bytes). use --force to force \
                                    upload",
                                   p.to_string_lossy(),
                                   size,
                                   limit)
                  .into());
              }
            }
          }
          Ok(p)
        })
        .map(|p| p.and_then(|f| self.read_file_to_pastefile(f)))
        .map(|r| r.map_err(|e| e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n")))
        .collect::<Vec<_>>();
      for res in results.iter().cloned() {
        if res.is_err() {
          return Err(res.err().unwrap().into());
        }
      }
      let mut pastes =
        results.iter().cloned().map(|r| r.unwrap()).filter(|p| !p.data.trim().is_empty()).collect::<Vec<_>>();
      self.handle_duplicate_file_names(&mut pastes);
      if !self.arguments.force {
        if let Some((name, pattern)) = self.check_for_disallowed_files(&pastes) {
          return Err(format!("\"{}\" is disallowed by the pattern \"{}\". use --force to force upload",
                             name,
                             pattern)
            .into());
        }
        try!(check_magic(self, &pastes));
      }
      pastes
    } else {
      let mut buffer = String::new();
      if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
        return Err(format!("error reading stdin: {}", e).into());
      }
      let name = arguments.name.clone().unwrap_or_else(|| String::from("stdin"));
      vec![PasteFile::new(name, buffer)]
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
      if names_map.contains_key(&name) {;
        let number = names_map.entry(name.clone()).or_insert(1);
        paste.name = Bins::add_number_to_string(&name, *number);
        *number += 1;
      }
      names_map.entry(name.clone()).or_insert(1);
    }
  }

  pub fn add_number_to_string(string: &str, num: i32) -> String {
    let (beginning, end) = {
      let path = Path::new(&string);
      let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_owned();
      let extension = path.extension().and_then(|s| s.to_str()).map_or_else(String::new, |s| String::from(".") + s);
      (stem, extension)
    };
    format!("{}_{}{}", beginning, num, end)
  }

  pub fn add_number_to_path(path: &PathBuf, num: i32) -> PathBuf {
    let (beginning, end) = {
      let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_owned();
      let extension = path.extension().and_then(|s| s.to_str()).map_or_else(String::new, |s| String::from(".") + s);
      (stem, extension)
    };
    let name = format!("{}_{}{}", beginning, num, end);
    let mut clone = path.clone();
    clone.set_file_name(name);
    clone
  }

  pub fn sanitize_path(path: &Path) -> Result<&str> {
    Ok(some_or_err!(path.components()
                      .filter_map(|s| {
                        match s {
                          Component::Normal(x) => Some(x),
                          _ => None
                        }
                      })
                      .last()
                      .and_then(|s| s.to_str()),
                    "file name had no valid path components".into()))
  }

  fn get_engine_for_url<'a>(&'a self, url: &'a Url) -> Result<&Box<Bin>> {
    let domain = some_or_err!(url.domain(), "input url had no domain".into());
    let engine = some_or_err!(engines::get_bin_by_domain(domain),
                              format!("could not find a bin for domain {}", domain).into());
    Ok(engine)
  }

  fn get_raw(&self, url_string: &str) -> Result<String> {
    let url = try!(network::parse_url(url_string.as_ref()));
    let url_clone = url.clone();
    let bin = try!(self.get_engine_for_url(&url_clone));
    if !bin.verify_url(&url_clone) {
      return Err(format!("invalid url for {}", bin.get_name()).into());
    }
    Ok(try!(bin.produce_raw_contents(self, &url)))
  }

  fn check_for_disallowed_files(&self, to_paste: &[PasteFile]) -> Option<(String, String)> {
    for pattern in self.config.get_general_disallowed_file_patterns().unwrap_or(&[]) {
      let pattern = match pattern.as_str() {
        Some(s) => s,
        None => continue,
      };
      for file in to_paste {
        if file.name.matches_pattern(pattern) {
          return Some((file.name.clone(), pattern.to_owned()));
        }
      }
    }
    None
  }

  pub fn get_output(&self) -> Result<String> {
    if let Some(ref input) = self.arguments.input {
      return self.get_raw(input);
    }
    let mut to_paste = try!(self.get_to_paste());
    let engine = try!(self.get_engine());
    let upload_url = if to_paste.len() > 1 {
      try!(engine.upload_all(self, to_paste))
    } else if to_paste.len() == 1 {
      let file = to_paste.pop().expect("len() == 1 but no element from pop()");
      try!(engine.upload_paste(self, file))
    } else {
      return Err("no files to upload".into());
    };
    if self.arguments.json {
      return Ok(try!(json::encode(&UploadResult {
        success: true,
        url: Some(upload_url.as_str()),
        bin: Some(engine.get_name())
      })));
    }
    Ok(upload_url.as_str().to_owned())
  }
}

#[derive(RustcEncodable)]
struct UploadResult<'a> {
  success: bool,
  url: Option<&'a str>,
  bin: Option<&'a str>
}

#[derive(Clone)]
pub struct FlexibleRange {
  pub ranges: Vec<Range<usize>>
}

impl FlexibleRange {
  pub fn parse<S: Into<String>>(string: S) -> Result<FlexibleRange> {
    let string = string.into();
    let parts = string.split(',').map(|s| s.trim());
    let mut range: Vec<Range<usize>> = Vec::new();
    for part in parts {
      let bounds = part.split('-').map(|s| s.trim()).collect::<Vec<_>>();
      match bounds.len() {
        1 => {
          let num = bounds[0];
          if num.is_empty() {
            return Err("empty part in range".into());
          }
          let num = try!(num.parse::<usize>());
          range.push(num..num + 1);
        }
        2 => {
          let lower = bounds[0];
          let upper = bounds[1];
          if lower.is_empty() || upper.is_empty() {
            return Err("incomplete part in range".into());
          }
          let lower: usize = try!(lower.parse());
          let upper: usize = try!(upper.parse());
          let r: Range<usize> = if lower > upper {
            lower + 1..upper
          } else {
            lower..upper + 1
          };
          range.push(r);
        }
        _ => return Err("too many dashes in range".into())
      }
    }
    Ok(FlexibleRange { ranges: range })
  }
}

impl Iterator for FlexibleRange {
  type Item = usize;

  fn next(&mut self) -> Option<Self::Item> {
    if self.ranges.len() < 1 {
      return None;
    }
    let n = {
      let mut range = &mut self.ranges[0];
      if range.start > range.end {
        let normalized = range.end..range.start;
        let ret = normalized.rev().next();
        if ret.is_some() {
          range.start -= 1;
        }
        ret
      } else {
        range.next()
      }
    };
    if n.is_none() {
      self.ranges.remove(0);
      return self.next();
    }
    n
  }
}

trait MatchesPattern<'a> {
  fn matches_pattern(&self, pattern: &'a str) -> bool;
}

impl<'a> MatchesPattern<'a> for String {
  fn matches_pattern(&self, pattern: &'a str) -> bool {
    if pattern == "*" || pattern == self {
      return true;
    }
    if self.is_empty() {
      return false;
    }
    let pattern_first = match pattern.chars().next() {
      Some(f) => f,
      None => return false,
    };
    let string_first = match self.chars().next() {
      Some(f) => f,
      None => return false,
    };
    if pattern_first == string_first {
      return (&self[1..]).to_owned().matches_pattern(&pattern[1..]);
    }
    if pattern_first == '*' {
      return self.matches_pattern(&pattern[1..]) || (&self[1..]).to_owned().matches_pattern(pattern);
    }
    false
  }
}
