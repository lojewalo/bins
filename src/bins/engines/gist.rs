use bins::error::*;
use bins::engines::{Bin, ConvertUrlsToRawUrls, Info, PasteContents, ProduceInfo, ProduceRawContent, ProduceRawInfo,
                    RemotePasteFile, UploadBatchContent, UploadContent, VerifyUrl};
use bins::network::download::{Downloader, ModifyDownloadRequest};
use bins::network::upload::{ModifyUploadRequest, Uploader};
use bins::network::{self, RequestModifiers};
use bins::{Bins, PasteFile};
use hyper::header::{Authorization, Basic, ContentType, Headers, UserAgent};
use hyper::status::StatusCode;
use hyper::Url;
use rustc_serialize::json::{self, Json};
use std::collections::BTreeMap;

lazy_static! {
  static ref GOOD_CHARS: &'static str = "abcdefghijklmnopqrstuvwxyz0123456789-_";
}

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
    let url = some_or_err!(gist.get("html_url").and_then(|r| r.as_string()),
                           "html_url_key was not present or was not a string".into());
    Ok(try!(network::parse_url(url)))
  }

  fn get_gist(&self, bins: &Bins, url: &Url) -> Result<GistUpload> {
    let id = some_or_err!(url.path_segments().and_then(|r| r.last()),
                          "could not get path of url".into());
    let url = try!(network::parse_url(format!("https://api.github.com/gists/{}", id)));
    let mut res = try!(self.download(bins, &url));
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

impl VerifyUrl for Gist {
  fn verify_url(&self, url: &Url) -> bool {
    let segments = self.segments(url);
    segments.len() == 1 || segments.len() == 2
  }
}

impl ConvertUrlsToRawUrls for Gist {
  fn convert_url_to_raw_url(&self, _: &Bins, _: &Url) -> Result<Url> {
    // this should never, ever be called
    Err("gist urls are not a one-to-one conversion (this is a bug)".into())
  }

  #[cfg_attr(feature = "clippy", allow(needless_return))]
  fn convert_urls_to_raw_urls(&self, bins: &Bins, urls: Vec<&Url>) -> Result<Vec<Url>> {
    if urls.len() != 1 {
      return Err("multiple gist urls given (this is a bug)".into());
    }
    let url = urls[0];
    let remote_upload: GistUpload = try!(self.get_gist(bins, &url));
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
  fn produce_raw_info(&self, bins: &Bins, url: &Url) -> Result<Info> {
    let mut info = try!(self.produce_info(bins, url));
    info.raw = true;
    Ok(info)
  }

  fn produce_raw_info_all(&self, bins: &Bins, urls: Vec<&Url>) -> Result<Vec<Info>> {
    let info: Vec<Info> = try!(urls.iter().map(|u| self.produce_raw_info(bins, u)).collect());
    Ok(info)
  }
}

impl ProduceInfo for Gist {
  fn produce_info(&self, bins: &Bins, url: &Url) -> Result<Info> {
    let gist = try!(self.get_gist(bins, url));
    let html_url = some_or_err!(gist.html_url, "no html_url from gist".into());
    let files: Result<Vec<RemotePasteFile>> = gist.files
      .iter()
      .map(|(n, g)| {
        // name, gist
        let new_url = try!(RemoteGistFile::get_html_url(&html_url, n));
        let raw_url = match &g.raw_url {
          &Some(ref s) => try!(network::parse_url(s.clone())),
          &None => return Err("a gist file did not have a raw_url (this is a bug)".into()),
        };
        Ok(RemotePasteFile {
          name: n.to_owned(),
          id: n.to_owned(),
          bin: self.get_name().to_owned(),
          url: new_url,
          raw_url: raw_url,
          contents: PasteContents {
            truncated: g.truncated,
            value: Some(g.content.clone())
          }
        })
      })
      .collect();
    Ok(Info {
      id: gist.id,
      name: gist.description,
      url: url.clone(),
      raw_url: None,
      raw: false,
      files: try!(files),
      index: None,
      contents: PasteContents::default(),
      bin: self.get_name().to_owned()
    })
  }
}

impl ProduceRawContent for Gist {}

impl Uploader for Gist {}

impl ModifyDownloadRequest for Gist {
  fn modify_request(&self, _: &Bins) -> Result<RequestModifiers> {
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
      if let Some(username) = bins.config.get_gist_username() {
        if let Some(token) = bins.config.get_gist_access_token() {
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
  id: String,
  files: BTreeMap<String, RemoteGistFile>,
  description: String,
  public: bool,
  html_url: Option<String>
}

impl GistUpload {
  fn new(id: Option<String>, description: Option<String>, public: bool) -> Self {
    let map = BTreeMap::new();
    GistUpload {
      id: id.unwrap_or_else(String::new),
      files: map,
      description: description.unwrap_or_else(String::new),
      public: public,
      html_url: None
    }
  }

  fn from(bins: &Bins, files: &[PasteFile]) -> Self {
    let mut gist = GistUpload::new(None, None, !bins.arguments.private);
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

impl RemoteGistFile {
  fn get_html_url(html_url: &str, name: &str) -> Result<Url> {
    let replaced: String = name.to_lowercase()
      .chars()
      .map(|c| if GOOD_CHARS.contains(c) {
        c
      } else {
        '-'
      })
      .collect();
    network::parse_url(format!("{}#file-{}", html_url, replaced))
  }
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
