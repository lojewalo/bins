use bins::engines::Engine;
use bins::PasteFile;
use config::types::Config;
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
  fn new(description: Option<String>, public: Option<bool>) -> Self {
    let map = HashMap::new();
    GistUpload {
      files: map,
      description: description.unwrap_or(String::from("")),
      public: public.unwrap_or(false)
    }
  }
}

impl<'a> From<&'a Vec<PasteFile>> for GistUpload {
  fn from(files: &'a Vec<PasteFile>) -> Self {
    let mut gist = GistUpload::new(None, None);
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
  fn upload(&self, config: &Config, data: &Vec<PasteFile>) -> Result<String, String> {
    let upload = GistUpload::from(data);
    let j = try!(json::encode(&upload).map_err(|e| e.to_string()));
    let client = Client::new();
    let mut res = try!({
      let mut builder = client
        .post("https://api.github.com/gists")
        .body(&j)
        .header(ContentType::json())
        .header(UserAgent(String::from("bins")));
      if let Some(username) = config.lookup_str("gist.username") {
        if let Some(token) = config.lookup_str("gist.access_token") {
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
      builder
        .send()
        .map_err(|e| e.to_string())
    });
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    if res.status != StatusCode::Created {
      println!("{}", s);
      return Err(String::from("gist could not be created"));
    }
    let raw_gist = try!(Json::from_str(&s).map_err(|e| e.to_string()));
    let gist = some_or_err!(raw_gist.as_object(), String::from("response was not a json object"));
    let html_url = some_or_err!(gist.get("html_url"), String::from("no html_url_key"));
    let url = some_or_err!(html_url.as_string(), String::from("html_url was not a string"));
    Ok(url.to_owned())
  }
}
