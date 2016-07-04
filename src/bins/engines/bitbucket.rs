use bins::{Bins, PasteFile};
use bins::engines::{Bin, ConvertUrlsToRawUrls, Info, PasteContents, ProduceInfo, ProduceRawContent, ProduceRawInfo,
                    RemotePasteFile, UploadBatchContent, UploadContent, VerifyUrl};
use bins::error::*;
use bins::network::download::{Downloader, ModifyDownloadRequest};
use bins::network::upload::{ModifyUploadRequest, Uploader};
use bins::network::{self, RequestModifiers};
use hyper::Url;
use hyper::client::Client;
use hyper::header::{Authorization, Basic, ContentType, Headers, UserAgent};
use hyper::mime::{Attr, Mime, SubLevel, TopLevel, Value};
use hyper::status::StatusCode;
use rand::{Rng, thread_rng};
use rustc_serialize::json::{self, Json};
use rustc_serialize::base64::{MIME, ToBase64};
use std::io::Read;
use std::collections::BTreeMap;

pub struct Bitbucket;

impl Bitbucket {
  pub fn new() -> Self {
    Bitbucket {}
  }

  fn upload_snippet(&self, bins: &Bins, data: Vec<PasteFile>) -> Result<Url> {
    let authorization = try!(self.authorization(bins));

    let boundary = self.random_boundary();
    let headers = self.prepare_headers(&boundary, authorization);
    let body = try!(self.prepare_body(bins, &data, &boundary));

    let client = Client::new();
    let builder = client.post("https://api.bitbucket.org/2.0/snippets")
      .headers(headers)
      .body(&body);

    let mut response = try!(builder.send().map_err(|e| e.to_string()));
    let mut response_body = String::new();
    try!(response.read_to_string(&mut response_body).map_err(|e| e.to_string()));
    if response.status != StatusCode::Created {
      let msg = format!("snippet could not be created, response: {}\n{}",
                        response.status,
                        response_body);
      return Err(msg.into());
    }

    let snippet = try!(Json::from_str(&response_body).map_err(|e| e.to_string()));
    let url = some_or_err!(snippet.find_path(&["links", "html", "href"])
                             .and_then(|j| j.as_string()),
                           "string links.html.href not found in response".into());
    network::parse_url(url)
  }

  fn get_snippet(&self, bins: &Bins, url: &Url) -> Result<Snippet> {
    let segments: Vec<_> = some_or_err!(url.path_segments(), "url has no path".into()).collect();
    if segments.len() < 3 || segments[0] != "snippets" {
      return Err("url path expected to be of form /snippets/{username}/{id}".into());
    }
    let username = segments[1];
    let id = segments[2];

    let api_url = try!(network::parse_url(format!("https://api.bitbucket.org/2.0/snippets/{}/{}", username, id)));
    let mut res = try!(self.download(bins, &api_url));
    let content = try!(network::read_response(&mut res));
    Ok(try!(json::decode(&content)))
  }

  fn random_boundary(&self) -> String {
    thread_rng().gen_ascii_chars().take(69).collect()
  }

  fn authorization(&self, bins: &Bins) -> Result<Authorization<Basic>> {
    let username = some_or_err!(bins.config.get_bitbucket_username(),
                                "no Bitbucket username set in configuration".into());
    let app_password = some_or_err!(bins.config.get_bitbucket_app_password(),
                                    "no Bitbucket app password set in configuration".into());
    if username.is_empty() || app_password.is_empty() {
      return Err("Bitbucket username or app_password in configuration was empty".into());
    }
    Ok(Authorization(Basic {
      username: username.to_string(),
      password: Some(app_password.to_string())
    }))
  }

  fn prepare_headers(&self, boundary: &str, authorization: Authorization<Basic>) -> Headers {
    let mut headers = Headers::new();
    let content_type = ContentType(Mime(TopLevel::Multipart,
                                        SubLevel::Ext("related".to_string()),
                                        vec![(Attr::Boundary, Value::Ext(boundary.to_string()))]));
    headers.set(content_type);
    headers.set_raw("MIME-Version", vec![b"1.0".to_vec()]);
    headers.set(UserAgent("bins".to_string()));
    headers.set(authorization);

    headers
  }

  fn prepare_body(&self, bins: &Bins, data: &[PasteFile], boundary: &str) -> Result<String> {
    let properties = SnippetProperties {
      title: "bins".to_string(),
      is_private: bins.arguments.private
    };
    let properties_json = try!(json::encode(&properties).map_err(|e| e.to_string()));

    let mut body = MultipartRelatedBody::new(boundary);
    body.add_json(&properties_json);
    for file in data {
      body.add_file(&file.name, file.data.as_bytes());
    }

    Ok(body.end())
  }
}

impl Bin for Bitbucket {
  fn get_name(&self) -> &str {
    "bitbucket"
  }

  fn get_domain(&self) -> &str {
    "bitbucket.org"
  }
}

impl VerifyUrl for Bitbucket {
  fn verify_url(&self, url: &Url) -> bool {
    let segments = self.segments(url);
    segments.len() > 2 && segments[0] == "snippets"
  }
}

impl ConvertUrlsToRawUrls for Bitbucket {
  fn convert_url_to_raw_url(&self, _: &Bins, _: &Url) -> Result<Url> {
    // this should never, ever be called
    Err("Bitbucket snippet URLs are not a one-to-one conversion (this is a bug)".into())
  }

  fn convert_urls_to_raw_urls(&self, bins: &Bins, urls: Vec<&Url>) -> Result<Vec<Url>> {
    if urls.len() != 1 {
      return Err("multiple Bitbucket snippet urls given (this is a bug)".into());
    }
    let url = urls[0];
    let snippet = try!(self.get_snippet(bins, &url));
    snippet.files
      .iter()
      .map(|(name, f)| {
        let link = some_or_err!(f.links.get("self").map(|l| l.href.to_string()),
                                format!("file {} had no self link", name).into());
        network::parse_url(link)
      })
      .collect()
  }
}

impl UploadContent for Bitbucket {
  fn upload_paste(&self, bins: &Bins, content: PasteFile) -> Result<Url> {
    self.upload_snippet(bins, vec![content])
  }
}

impl UploadBatchContent for Bitbucket {
  fn upload_all(&self, bins: &Bins, content: Vec<PasteFile>) -> Result<Url> {
    self.upload_snippet(bins, content)
  }
}

impl ProduceRawInfo for Bitbucket {
  fn produce_raw_info(&self, bins: &Bins, url: &Url) -> Result<Info> {
    let mut info = try!(self.produce_info(bins, url));
    info.raw = true;
    Ok(info)
  }

  fn produce_raw_info_all(&self, bins: &Bins, urls: Vec<&Url>) -> Result<Vec<Info>> {
    let files: Vec<Info> = try!(urls.iter().map(|u| self.produce_raw_info(bins, u)).collect());
    Ok(files)
  }
}

impl ProduceInfo for Bitbucket {
  fn produce_info(&self, bins: &Bins, url: &Url) -> Result<Info> {
    let snippet = try!(self.get_snippet(bins, url));
    let files: Result<Vec<RemotePasteFile>> = snippet.files
      .iter()
      .map(|(name, file)| {
        let link = some_or_err!(file.links.get("html"),
                                format!("file {} had no html link", name).into());
        let file_url = try!(network::parse_url(link.href.to_string()));
        let link = some_or_err!(file.links.get("self").map(|l| l.href.to_string()),
                                format!("file {} had no self link", name).into());
        let raw_url = try!(network::parse_url(link));
        Ok(RemotePasteFile {
          name: name.to_owned(),
          id: name.to_owned(),
          bin: self.get_name().to_owned(),
          url: file_url,
          raw_url: raw_url,
          contents: PasteContents::default()
        })
      })
      .collect();
    Ok(Info {
      id: snippet.id,
      name: snippet.title,
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

impl ProduceRawContent for Bitbucket {}

impl Uploader for Bitbucket {}

impl ModifyUploadRequest for Bitbucket {}

impl Downloader for Bitbucket {}

impl ModifyDownloadRequest for Bitbucket {
  fn modify_request(&self, bins: &Bins) -> Result<RequestModifiers> {
    let authorization = try!(self.authorization(bins));
    let mut headers = Headers::new();
    headers.set(UserAgent(String::from("bins")));
    headers.set(authorization);
    Ok(RequestModifiers { headers: Some(headers), ..RequestModifiers::default() })
  }
}

unsafe impl Sync for Bitbucket {}


#[derive(RustcDecodable)]
struct Snippet {
  id: String,
  title: String,
  files: BTreeMap<String, File>
}

#[derive(RustcDecodable)]
struct File {
  links: BTreeMap<String, Link>
}

#[derive(RustcDecodable)]
struct Link {
  href: String
}


#[derive(RustcEncodable)]
struct SnippetProperties {
  title: String,
  is_private: bool
}

struct MultipartRelatedBody<'a> {
  boundary: &'a str,
  content: String
}

impl<'a> MultipartRelatedBody<'a> {
  fn new(boundary: &str) -> MultipartRelatedBody {
    MultipartRelatedBody {
      boundary: boundary,
      content: String::new()
    }
  }

  fn add_json(&mut self, json: &str) {
    self.add_boundary();
    self.add_line("Content-Type: application/json; charset=\"utf-8\"");
    self.add_line("MIME-Version: 1.0");
    self.add_line("Content-ID: snippet");
    self.end_line();

    self.add_line(json);
    self.end_line();
  }

  fn add_file(&mut self, filename: &str, content: &[u8]) {
    self.add_boundary();
    self.add_line("Content-Type: text/plain; charset=\"utf-8\"");
    self.add_line("MIME-Version: 1.0");
    self.add_line("Content-Transfer-Encoding: base64");

    self.add("Content-ID: \"");
    self.add(filename);
    self.add("\"");
    self.end_line();

    self.add("Content-Disposition: attachment; filename=\"");
    self.add(filename);
    self.add("\"");
    self.end_line();

    self.end_line();

    self.content.push_str(&content.to_base64(MIME));
    self.end_line();
  }

  fn end(mut self) -> String {
    self.content.push_str("--");
    self.content.push_str(self.boundary);
    self.content.push_str("--");
    self.end_line();
    self.content
  }

  fn add_boundary(&mut self) {
    self.content.push_str("--");
    self.content.push_str(self.boundary);
    self.end_line();
  }

  fn add(&mut self, s: &str) {
    self.content.push_str(s);
  }

  fn add_line(&mut self, line: &str) {
    self.content.push_str(line);
    self.end_line();
  }

  fn end_line(&mut self) {
    self.content.push_str("\r\n");
  }
}
