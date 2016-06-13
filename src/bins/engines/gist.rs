use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use bins::engines::indexed::{IndexedDownload, DownloadsFile};
use bins::configuration::BetterLookups;
use hyper::client::Client;
use hyper::header::{Headers, ContentType, UserAgent, Authorization, Basic};
use hyper::status::StatusCode;
use std::collections::HashMap;
use std::io::Read;
use rustc_serialize::json::{self, Json};
use hyper::Url;

#[derive(RustcEncodable)]
struct GistUpload {
  files: HashMap<String, GistFile>,
  description: String,
  public: bool
}

impl GistUpload {
  fn new(description: Option<String>, public: bool) -> Self {
    let map = HashMap::new();
    GistUpload {
      files: map,
      description: description.unwrap_or_else(String::new),
      public: public
    }
  }

  fn from(bins: &Bins, files: &[PasteFile]) -> Self {
    let mut gist = GistUpload::new(None, !bins.arguments.private);
    for file in files {
      gist.files.insert(file.name.clone(), GistFile::from(file.data.clone()));
    }
    gist
  }
}

#[derive(RustcEncodable)]
struct GistFile {
  content: String
}

impl From<String> for GistFile {
  fn from(string: String) -> Self {
    GistFile { content: string }
  }
}

pub struct Gist;

impl Gist {
  pub fn new() -> Self {
    Gist { }
  }
}

impl Engine for Gist {
  fn upload(&self, bins: &Bins, data: &[PasteFile]) -> Result<String> {
    let upload = GistUpload::from(bins, data);
    let j = try!(json::encode(&upload).map_err(|e| e.to_string()));
    let client = Client::new();
    let mut res = try!({
      let mut builder = client
        .post("https://api.github.com/gists")
        .body(&j)
        .header(ContentType::json())
        .header(UserAgent(String::from("bins")));
      if bins.arguments.auth {
        if let Some(username) = bins.config.lookup_str("gist.username") {
          if let Some(token) = bins.config.lookup_str("gist.access_token") {
            if !username.is_empty() && !token.is_empty() {
              builder = builder.header(
                Authorization(
                  Basic {
                    username: username.to_owned(),
                    password: Some(token.to_owned())
                  }
                )
              );
            }
          }
        }
      }
      builder
        .send()
        .map_err(|e| e.to_string())
    });
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    if res.status != StatusCode::Created {
      println!("{}", s);
      return Err("paste could not be created".into());
    }
    let raw_gist = try!(Json::from_str(&s).map_err(|e| e.to_string()));
    let gist = some_or_err!(raw_gist.as_object(), "response was not a json object".into());
    let html_url = some_or_err!(gist.get("html_url"), "no html_url_key".into());
    let url = some_or_err!(html_url.as_string(), "html_url was not a string".into());
    Ok(url.to_owned())
  }

  fn get_raw(&self, bins: &Bins, url: &mut Url) -> Result<String> {
    let id = some_or_err!(some_or_err!(url.path_segments(), "could not get path of url".into()).last(), "could not get last path of url".into());
    if bins.arguments.files.len() > 1 {
      return Err("currently, only one file is able to be retrieved in input mode".into());
    }
    let target_file = bins.arguments.files.get(0);
    let client = Client::new();
    let mut res = try!(client
      .get(&format!("https://api.github.com/gists/{}", id))
      .header(UserAgent(String::from("bins")))
      .send()
      .map_err(|e| e.to_string()));
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    if res.status != StatusCode::Ok {
      println!("{}", s);
      return Err("status was not ok".into());
    }
    let raw_gist = try!(Json::from_str(&s).map_err(|e| e.to_string()));
    let gist = some_or_err!(raw_gist.as_object(), "response was not a json object".into());
    let files = some_or_err!(some_or_err!(gist.get("files"), "no files".into()).as_object(), "files was not a json object".into());
    let keys = files.keys().cloned().map(|s| s.to_lowercase()).collect::<Vec<_>>();
    if keys.len() < 1 {
      return Err("gist had no files".into());
    }
    if keys.len() > 1 && target_file.is_none() {
      let file_names = keys.iter().map(|s| String::from("  ") + s).collect::<Vec<_>>().join("\n");
      let message = format!("gist had more than one file, but no target file was specified\n\nfiles available:\n{}", file_names);
      return Err(message.into());
    }
    let target = target_file.unwrap_or(&keys[0]).to_lowercase();
    if !keys.contains(&target) {
      return Err("gist did not contain file".into());
    }
    let file = some_or_err!(some_or_err!(files.get(&target), format!("could not find {}", target).into()).as_object(), "file was not a json object".into());
    let raw_url = some_or_err!(some_or_err!(file.get("raw_url"), "no raw_url".into()).as_string(), "raw_url was not a string".into());
    let download = IndexedDownload {
      url: raw_url.to_owned(),
      headers: Headers::new(),
      target: None
    };
    download.download()
  }
}
