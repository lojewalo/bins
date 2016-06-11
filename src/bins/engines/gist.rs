use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use hyper::client::Client;
use hyper::header::{ContentType, UserAgent, Authorization, Basic};
use hyper::status::StatusCode;
use std::collections::HashMap;
use std::io::Read;
use rustc_serialize::json::{self, Json};

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
}
