use bins::error::*;
use bins::configuration::BetterLookups;
use bins::engines::{Bin, ConvertUrlsToRawUrls, ProduceInfo, ProduceRawContent, ProduceRawInfo, RemotePasteFile,
                    UploadBatchContent, UploadContent};
use bins::network::download::{Downloader, ModifyDownloadRequest};
use bins::network::upload::{ModifyUploadRequest, Uploader};
use bins::network::{self, RequestModifiers};
use bins::{Bins, PasteFile};
use hyper::header::{Authorization, Basic, ContentType, Headers, UserAgent};
use hyper::status::StatusCode;
use hyper::Url;
use rustc_serialize::json::{self, Json};
use std::collections::BTreeMap;

pub struct Gist;

impl Gist {
  pub fn new() -> Self {
    Gist {}
  }

  fn upload_gist(&self, bins: &Bins, content: Vec<PasteFile>) -> Result<Url> {
    let upload = GistUpload::from(bins, &*content);
    let j = try!(json::encode(&upload).map_err(|e| e.to_string()));
    let url = try!(network::parse_url("https://api.github.com/gists"));
    let mut res = try!(self.upload(&url,
                                   bins,
                                   &PasteFile {
                                     name: "json".to_owned(),
                                     data: j
                                   }));
    let s = try!(network::read_response(&mut res));
    if res.status != StatusCode::Created {
      println!("{}", s);
      return Err("paste could not be created".into());
    }
    let raw_gist = try!(Json::from_str(&s).map_err(|e| e.to_string()));
    let gist = some_or_err!(raw_gist.as_object(),
                            "response was not a json object".into());
    let html_url = some_or_err!(gist.get("html_url"), "no html_url_key".into());
    let url = some_or_err!(html_url.as_string(), "html_url was not a string".into());
    Ok(try!(network::parse_url(url)))
  }

  fn get_gist(&self, url: &Url) -> Result<GistUpload> {
    let id = some_or_err!(some_or_err!(url.path_segments(), "could not get path of url".into()).last(),
                          "could not get last path of url".into());
    let url = try!(network::parse_url(format!("https://api.github.com/gists/{}", id)));
    let mut res = try!(self.download(&url));
    let content = try!(network::read_response(&mut res));
    Ok(try!(json::decode(&content)))
  }
}

impl Bin for Gist {
  fn get_name(&self) -> &str {
    "gist"
  }

  fn get_domain(&self) -> &str {
    "gist.github.com"
  }
}

impl ConvertUrlsToRawUrls for Gist {
  fn convert_url_to_raw_url(&self, _: &Url) -> Result<Url> {
    // this should never, ever be called
    Err("gist urls are not a one-to-one conversion (this is a bug)".into())
  }

  fn convert_urls_to_raw_urls(&self, urls: Vec<&Url>) -> Result<Vec<Url>> {
    if urls.len() != 1 {
      return Err("multiple gist urls given (this is a bug)".into());
    }
    let url = urls[0];
    let remote_upload: GistUpload = try!(self.get_gist(&url));
    some_or_err!(remote_upload.files.iter().map(|(_, r)| r.raw_url.clone().map(network::parse_url)).collect(),
                 "a file in the gist did not have a raw url".into())
  }
}

impl UploadContent for Gist {
  fn upload_paste(&self, bins: &Bins, content: PasteFile) -> Result<Url> {
    self.upload_gist(bins, vec![content])
  }
}

impl UploadBatchContent for Gist {
  fn upload_all(&self, bins: &Bins, content: Vec<PasteFile>) -> Result<Url> {
    self.upload_gist(bins, content)
  }
}

impl ProduceRawInfo for Gist {
  fn produce_raw_info(&self, _: &Bins, url: &Url) -> Result<Vec<RemotePasteFile>> {
    let raw_urls = try!(self.convert_urls_to_raw_urls(vec![url]));
    Ok(try!(raw_urls.iter()
      .map(|u| {
        let name = some_or_err!(u.path_segments().and_then(|s| s.last()),
                                "paste url was a root url");
        Ok(RemotePasteFile {
          name: name.to_owned(),
          url: u.clone(),
          contents: None
        })
      })
      .collect()))
  }

  fn produce_raw_info_all(&self, bins: &Bins, urls: Vec<&Url>) -> Result<Vec<RemotePasteFile>> {
    let info: Vec<Vec<RemotePasteFile>> = try!(urls.iter().map(|u| self.produce_raw_info(bins, u)).collect());
    Ok(info.into_iter().flat_map(|v| v).collect())
  }
}

impl ProduceInfo for Gist {
  fn produce_info(&self, _: &Bins, url: &Url) -> Result<Vec<RemotePasteFile>> {
    lazy_static! {
      static ref GOOD_CHARS: &'static str = "abcdefghijklmnopqrstuvwxyz0123456789-_";
    }
    let gist = try!(self.get_gist(url));
    let html_url = some_or_err!(gist.html_url, "no html_url from gist".into());
    gist.files
      .iter()
      .map(|(n, g)| {
        let replaced: String = n.to_lowercase()
          .chars()
          .map(|c| if GOOD_CHARS.contains(c) {
            c
          } else {
            '-'
          })
          .collect();
        let new_url = try!(network::parse_url(format!("{}#file-{}", html_url, replaced)));
        Ok(RemotePasteFile {
          name: n.to_owned(),
          url: new_url,
          contents: if !g.truncated {
            Some(g.content.clone())
          } else {
            None
          }
        })
      })
      .collect()
  }
}

impl ProduceRawContent for Gist {}

impl Uploader for Gist {}

impl ModifyDownloadRequest for Gist {
  fn modify_request(&self) -> Result<RequestModifiers> {
    let mut headers = Headers::new();
    headers.set(UserAgent(String::from("bins")));
    Ok(RequestModifiers { headers: Some(headers), ..RequestModifiers::default() })
  }
}

impl ModifyUploadRequest for Gist {
  fn modify_request<'a>(&'a self, bins: &Bins, content: &PasteFile) -> Result<RequestModifiers> {
    let mut headers = Headers::new();
    headers.set(ContentType::json());
    headers.set(UserAgent(String::from("bins")));
    if bins.arguments.auth {
      if let Some(username) = bins.config.lookup_str("gist.username") {
        if let Some(token) = bins.config.lookup_str("gist.access_token") {
          if !username.is_empty() && !token.is_empty() {
            headers.set(Authorization(Basic {
              username: username.to_owned(),
              password: Some(token.to_owned())
            }));
          }
        }
      }
    }
    Ok(RequestModifiers {
      body: Some(content.data.clone()),
      headers: Some(headers)
    })
  }
}

impl Downloader for Gist {}

unsafe impl Sync for Gist {}

#[derive(RustcEncodable, RustcDecodable)]
struct GistUpload {
  files: BTreeMap<String, RemoteGistFile>,
  description: String,
  public: bool,
  html_url: Option<String>
}

impl GistUpload {
  fn new(description: Option<String>, public: bool) -> Self {
    let map = BTreeMap::new();
    GistUpload {
      files: map,
      description: description.unwrap_or_else(String::new),
      public: public,
      html_url: None
    }
  }

  fn from(bins: &Bins, files: &[PasteFile]) -> Self {
    let mut gist = GistUpload::new(None, !bins.arguments.private);
    for file in files {
      gist.files.insert(file.name.clone(), RemoteGistFile::from(file.data.clone()));
    }
    gist
  }
}

#[derive(RustcEncodable, RustcDecodable)]
struct RemoteGistFile {
  content: String,
  raw_url: Option<String>,
  truncated: bool
}

impl From<String> for RemoteGistFile {
  fn from(string: String) -> Self {
    RemoteGistFile {
      content: string,
      raw_url: None,
      truncated: false
    }
  }
}
