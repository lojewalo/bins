use url::Url;
use hyper::Client;
use hyper::header::{Authorization, Basic, ContentType, Headers, UserAgent};
use hyper::mime::{Attr, Mime, SubLevel, TopLevel, Value};
use hyper::status::StatusCode;
use rand::{Rng, thread_rng};
use serde_json;
use base64;

use lib::*;
use lib::Result;
use lib::error::*;
use lib::files::*;
use config::{Config, CommandLineOptions};

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

pub struct Bitbucket {
  config: Arc<Config>,
  cli: Arc<CommandLineOptions>,
  client: Client
}

impl Bitbucket {
  pub fn new(config: Arc<Config>, cli: Arc<CommandLineOptions>) -> Bitbucket {
    Bitbucket {
      config: config,
      cli: cli,
      client: ::new_client()
    }
  }

  fn get_snippet(&self, id: &str) -> Result<Snippet> {
    let url_str = format!("https://bitbucket.org/snippets/{}", id);
    let url = Url::parse(&url_str).map_err(ErrorKind::UrlParse)?;
    let segments: Vec<_> = match url.path_segments() {
      Some(p) => p.collect(),
      None => bail!("could not parse ID from URL")
    };
    if segments.len() < 3 || segments[0] != "snippets" {
      bail!("url path expected to be of form /snippets/{username}/{id}");
    }
    let username = segments[1];
    let id = segments[2];

    let api_url = Url::parse(&format!("https://api.bitbucket.org/2.0/snippets/{}/{}", username, id)).map_err(ErrorKind::UrlParse)?;
    let mut res = self.client.get(api_url)
      .header(UserAgent(format!("bins/{}", crate_version!())))
      .header(self.authorization()?)
      .send()
      .map_err(ErrorKind::Http)?;
    let mut content = String::new();
    res.read_to_string(&mut content)?;
    Ok(serde_json::from_str(&content)?)
  }

  fn random_boundary(&self) -> String {
    thread_rng().gen_ascii_chars().take(69).collect()
  }

  fn authorization(&self) -> Result<Authorization<Basic>> {
    let config_values = (&self.config.bitbucket.username, &self.config.bitbucket.app_password);
    let (username, app_password) = match config_values {
      (&Some(ref u), &Some(ref ap)) if !u.is_empty() && !ap.is_empty() => (u, ap),
      _ => bail!("no bitbucket username/app password set")
    };
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

  fn prepare_body(&self, data: &[UploadFile], boundary: &str) -> Result<String> {
    let properties = SnippetProperties {
      title: "bins".to_string(),
      is_private: self.cli.private.unwrap_or_default()
    };
    let properties_json = serde_json::to_string(&properties).map_err(ErrorKind::Json)?;

    let mut body = MultipartRelatedBody::new(boundary);
    body.add_json(&properties_json);
    for file in data {
      body.add_file(&file.name, file.content.as_bytes());
    }

    Ok(body.end())
  }
}

impl Bin for Bitbucket {
  fn name(&self) -> &str {
    "bitbucket"
  }

  fn html_host(&self) -> &str {
    "bitbucket.org"
  }

  fn raw_host(&self) -> &str {
    "bitbucket.org"
  }
}

impl ManagesUrls for Bitbucket {}

impl CreatesUrls for Bitbucket {}

impl FormatsUrls for Bitbucket {}

impl FormatsHtmlUrls for Bitbucket {
  fn format_html_url(&self, _: &str) -> Option<String> {
    None
  }
}

impl FormatsRawUrls for Bitbucket {
  fn format_raw_url(&self, _: &str) -> Option<String> {
    None
  }
}

impl CreatesHtmlUrls for Bitbucket {
  fn create_html_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let snippet = self.get_snippet(id)?;
    snippet.files
      .iter()
      .map(|(name, f)| {
        let s = f.links.get("self").map(|l| l.href.to_string());
        match s {
          Some(x) => Ok(PasteUrl::raw(Some(PasteFileName::Explicit(name.clone())), x)),
          None => Err(ErrorKind::InvalidResponse.into())
        }
      })
      .collect()
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    let mut url = option!(Url::parse(url).ok());
    url.set_fragment(None);
    url.set_query(None);
    let segments: Vec<_> = option!(url.path_segments()).collect();
    let size = segments.len();
    Some(format!("{}/{}", segments[size - 2], segments[size - 1]))
  }
}

impl CreatesRawUrls for Bitbucket {
  fn create_raw_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let snippet = self.get_snippet(id)?;
    snippet.files
      .iter()
      .map(|(name, f)| {
        let s = f.links.get("self").map(|l| l.href.to_string());
        match s {
          Some(x) => Ok(PasteUrl::raw(Some(PasteFileName::Explicit(name.clone())), x)),
          None => Err(ErrorKind::InvalidResponse.into())
        }
      })
      .collect()
  }

  fn id_from_raw_url(&self, str_url: &str) -> Option<String> {
    let mut url = option!(Url::parse(str_url).ok());
    url.set_fragment(None);
    url.set_query(None);
    let segments: Vec<&str> = option!(url.path_segments()).collect();
    if let Some(first) = segments.get(0) {
      if *first != "!api" {
        return self.id_from_html_url(str_url);
      }
    }
    Some(format!("{}/{}", segments[3], segments[4]))
  }
}

impl HasFeatures for Bitbucket {
  fn features(&self) -> Vec<BinFeature> {
    vec![BinFeature::Public,
         BinFeature::Private,
         BinFeature::Authed,
         BinFeature::MultiFile,
         BinFeature::SingleNaming]
  }
}

impl Uploads for Bitbucket {
  fn upload(&self, contents: &[UploadFile], _: bool) -> Result<Vec<PasteUrl>> {
    let authorization = self.authorization()?;

    let boundary = self.random_boundary();
    let headers = self.prepare_headers(&boundary, authorization);
    let body = self.prepare_body(contents, &boundary)?;

    let mut response = self.client.post("https://api.bitbucket.org/2.0/snippets")
      .headers(headers)
      .body(&body)
      .send()
      .map_err(ErrorKind::Http)?;

    let mut response_body = String::new();
    response.read_to_string(&mut response_body).map_err(ErrorKind::Io)?;
    if response.status != StatusCode::Created {
      return Err(ErrorKind::BinError(response_body).into());
    }

    let snippet: serde_json::Value = serde_json::from_str(&response_body).map_err(ErrorKind::Json)?;
    match snippet.pointer("/links/html/href") {
      Some(h) if h.is_string() => Ok(vec![PasteUrl::html(None, h.as_str().unwrap().to_string())]),
      _ => Err(ErrorKind::InvalidResponse.into())
    }
  }
}

impl HasClient for Bitbucket {
  fn client(&self) -> &Client {
    &self.client
  }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Snippet {
  id: String,
  title: String,
  files: BTreeMap<String, File>
}

#[derive(Deserialize)]
struct File {
  links: BTreeMap<String, Link>
}

#[derive(Deserialize)]
struct Link {
  href: String
}

#[derive(Debug, Serialize)]
struct SnippetProperties {
  title: String,
  is_private: bool
}

#[derive(Debug)]
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

    self.content.push_str(&base64::encode(content));
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
