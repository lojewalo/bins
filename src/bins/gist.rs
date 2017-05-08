use url::Url;
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_openssl::OpensslClient;
use hyper::client::RequestBuilder;
use hyper::header::{Headers, ContentType, UserAgent, Authorization, Basic};
use serde_json;

use lib::*;
use lib::Result;
use lib::error::*;
use lib::files::*;
use config::{Config, CommandLineOptions};

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

const GOOD_CHARS: &'static str = "abcdefghijklmnopqrstuvwxyz0123456789-_";

pub struct Gist {
  config: Arc<Config>,
  cli: Arc<CommandLineOptions>,
  client: Client
}

impl Gist {
  pub fn new(config: Arc<Config>, cli: Arc<CommandLineOptions>) -> Gist {
    let ssl = OpensslClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);
    Gist {
      config: config,
      cli: cli,
      client: client
    }
  }

  fn add_headers<'a>(&self, rb: RequestBuilder<'a>) -> RequestBuilder<'a> {
    let mut headers = Headers::new();
    headers.set(ContentType::json());
    headers.set(UserAgent(format!("bins/{}", crate_version!())));
    if let Some(true) = self.cli.authed.or(self.config.defaults.authed) {
      if let Some(ref username) = self.config.gist.username {
        if let Some(ref access_token) = self.config.gist.access_token {
          if !username.is_empty() && !access_token.is_empty() {
            headers.set(Authorization(Basic {
              username: username.to_owned(),
              password: Some(access_token.to_owned())
            }));
          }
        }
      }
    }
    rb.headers(headers)
  }

  fn get_gist(&self, id: &str) -> Result<RemoteGistPaste> {
    debug!("getting gist for ID {}", id);
    let builder = self.client.get(&format!("https://api.github.com/gists/{}", id));
    let mut res = self.add_headers(builder).send().map_err(BinsError::Http)?;
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      return Err(BinsError::InvalidStatus(res.status_raw().0, Some(content)));
    }
    serde_json::from_str(&content).map_err(BinsError::Json)
  }
}

impl Bin for Gist {
  fn name(&self) -> &str {
    "gist"
  }

  fn html_host(&self) -> &str {
    "gist.github.com"
  }

  fn raw_host(&self) -> &str {
    "gist.githubusercontent.com"
  }
}

impl ManagesUrls for Gist {}

impl CreatesUrls for Gist {}

impl FormatsUrls for Gist {}

impl FormatsHtmlUrls for Gist {
  fn format_html_url(&self, _: &str) -> Option<String> {
    None
  }
}

impl FormatsRawUrls for Gist {
  fn format_raw_url(&self, _: &str) -> Option<String> {
    None
  }
}

impl CreatesHtmlUrls for Gist {
  fn create_html_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let gist = self.get_gist(id)?;
    let urls: Vec<PasteUrl> = gist.files.iter()
      .map(|(name, _)| PasteUrl::html(
        Some(PasteFileName::Explicit(name.clone())),
        format!("https://gist.github.com/{}/#file-{}",
          id,
          name.chars()
            .map(|c| c.to_lowercase().collect::<String>())
            .map(|c| if GOOD_CHARS.contains(&c) { c } else { "-".to_owned() })
            .collect::<String>())))
      .collect();
    Ok(urls)
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    let mut url = option!(Url::parse(url).ok());
    url.set_fragment(None);
    url.set_query(None);
    let segments = option!(url.path_segments());
    segments.last().map(|x| x.to_owned())
  }
}

impl CreatesRawUrls for Gist {
  fn create_raw_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let gist = self.get_gist(id)?;
    let urls: Option<Vec<PasteUrl>> = gist.files.iter()
      .map(|(name, file)| file.raw_url.clone()
        .map(|raw_url| PasteUrl::raw(Some(PasteFileName::Explicit(name.clone())), raw_url)))
      .collect();
    match urls {
      Some(u) => Ok(u),
      None => Err(BinsError::InvalidResponse)
    }
  }

  fn id_from_raw_url(&self, url: &str) -> Option<String> {
    let mut url = option!(Url::parse(url).ok());
    url.set_fragment(None);
    url.set_query(None);
    let segments: Vec<&str> = option!(url.path_segments()).collect();
    segments.get(1).map(|x| x.to_string())
  }
}

impl HasFeatures for Gist {
  fn features(&self) -> Vec<BinFeature> {
    vec![BinFeature::Public,
      BinFeature::Private,
      BinFeature::Authed,
      BinFeature::Anonymous,
      BinFeature::MultiFile,
      BinFeature::SingleNaming]
  }
}

impl Uploads for Gist {
  fn upload(&self, contents: &[UploadFile], _: bool) -> Result<Vec<PasteUrl>> {
    let mut files = BTreeMap::new();
    for file in contents {
      files.insert(file.name.clone(), UploadGistFile { content: file.content.clone() });
    }
    let upload_file = UploadGistPaste {
      public: self.cli.private.or(self.config.defaults.private).map(|x| !x).unwrap_or(false),
      files: files
    };
    let upload_json = serde_json::to_string(&upload_file).map_err(BinsError::Json)?;
    let builder = self.client.post("https://api.github.com/gists").body(&upload_json);
    let mut res = self.add_headers(builder).send().map_err(BinsError::Http)?;
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    if res.status != ::hyper::status::StatusCode::Created {
      return Err(BinsError::BinError(content));
    }
    let paste: RemoteGistPaste = serde_json::from_str(&content).map_err(BinsError::Json)?;
    match paste.html_url {
      Some(u) => Ok(vec![PasteUrl::html(None, u)]),
      None => Err(BinsError::InvalidResponse)
    }
  }
}

impl HasClient for Gist {
  fn client(&self) -> &Client {
    &self.client
  }
}

#[derive(Debug, Deserialize)]
struct RemoteGistPaste {
  id: String,
  files: BTreeMap<String, RemoteGistFile>,
  description: Option<String>,
  public: bool,
  html_url: Option<String>
}

#[derive(Debug, Deserialize)]
struct RemoteGistFile {
  content: String,
  raw_url: Option<String>,
  truncated: bool
}

#[derive(Debug, Serialize)]
struct UploadGistPaste {
  public: bool,
  files: BTreeMap<String, UploadGistFile>
}

#[derive(Debug, Serialize)]
struct UploadGistFile {
  content: String
}
