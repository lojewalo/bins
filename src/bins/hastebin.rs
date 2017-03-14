use url::Url;
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_openssl::OpensslClient;
use serde_json;
use serde_json::Result as JsonResult;

use lib::{Bin, BinFeature, ManagesUrls, ManagesHtmlUrls, ManagesRawUrls, UploadsSingleFiles, HasClient, HasFeatures, PasteUrl};
use lib::Result;
use lib::error::*;
use lib::files::*;
use config::{Config, CommandLineOptions};

use std::io::Read;
use std::sync::Arc;

pub struct Hastebin {
  config: Arc<Config>,
  cli: Arc<CommandLineOptions>,
  client: Client
}

impl Hastebin {
  pub fn new(config: Arc<Config>, cli: Arc<CommandLineOptions>) -> Hastebin {
    let ssl = OpensslClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);
    Hastebin {
      config: config,
      cli: cli,
      client: client
    }
  }

  fn id_from_url(&self, url: &str) -> Option<String> {
    let url = option!(Url::parse(url).ok());
    let segments = option!(url.path_segments());
    let last_segment = option!(segments.last());
    last_segment.split('.').next().map(|x| x.to_owned())
  }

  fn format_raw_url(&self, id: &str) -> String {
    format!("https://hastebin.com/raw/{}", id)
  }

  fn format_html_url(&self, id: &str) -> String {
    format!("https://hastebin.com/{}", id)
  }
}

impl Bin for Hastebin {
  fn name(&self) -> &str {
    "hastebin"
  }

  fn html_host(&self) -> &str {
    "hastebin.com"
  }

  fn raw_host(&self) -> &str {
    "hastebin.com"
  }
}

impl ManagesUrls for Hastebin {}

impl ManagesHtmlUrls for Hastebin {
  fn create_html_url(&self, id: &str, _: &[&str]) -> Result<Vec<PasteUrl>> {
    Ok(vec![PasteUrl::html(None, self.format_html_url(id))])
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl ManagesRawUrls for Hastebin {
  fn create_raw_url(&self, id: &str, _: &[&str]) -> Result<Vec<PasteUrl>> {
    let raw_url = self.format_raw_url(id);
    let mut res = self.client.get(&raw_url).send().map_err(BinsError::Http)?;
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    let parsed: serde_json::Result<Vec<IndexedFile>> = serde_json::from_str(&content);
    match parsed {
      Ok(is) => {
        debug!("file was an index, so checking its urls");
        let ids: Option<Vec<(String, String)>> = is.iter().map(|x| self.id_from_html_url(&x.url).map(|i| (x.name.clone(), i))).collect();
        let ids = match ids {
          Some(i) => i,
          None => {
            debug!("could not parse an ID from one of the URLs in the index");
            return Err(BinsError::Other);
          }
        };
        Ok(ids.into_iter().map(|(name, id)| PasteUrl::raw(Some(DownloadedFileName::Explicit(name)), self.format_raw_url(&id))).collect())
      },
      Err(_) => Ok(vec![PasteUrl::Downloaded(raw_url, DownloadedFile::new(DownloadedFileName::Guessed(id.to_owned()), content))])
    }
  }

  fn id_from_raw_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl HasFeatures for Hastebin {
  fn features(&self) -> Vec<BinFeature> {
    vec![BinFeature::Public, BinFeature::Anonymous]
  }
}

impl UploadsSingleFiles for Hastebin {
  fn upload_single(&self, content: &UploadFile) -> Result<String> {
    debug!(target: "hastebin", "uploading single file");
    let mut res = self.client.post("https://hastebin.com/documents")
      .body(&content.content)
      .send()
      .map_err(BinsError::Http)?;
    debug!(target: "hastebin", "res: {:?}", res);
    if res.status.class().default_code() != ::hyper::Ok {
      return Err(BinsError::Http(::hyper::Error::Status));
    }
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    debug!(target: "hastebin", "content: {}", content);
    let success: JsonResult<HastebinSuccess> = serde_json::from_str(&content);
    debug!(target: "hastebin", "success parse: {:?}", success);
    if let Ok(success) = success {
      debug!(target: "hastebin", "upload was a success. creating html url");
      return Ok((&self.create_html_url(&success.key, &[]).unwrap()[0]).url().to_owned());
    }
    debug!(target: "hastebin", "parse was a failure, try to parse as error");
    let error: JsonResult<HastebinError> = serde_json::from_str(&content);
    debug!(target: "hastebin", "error parse: {:?}", error);
    match error {
      Ok(e) => Err(BinsError::BinError(e.error)),
      Err(_) => Err(BinsError::InvalidResponse)
    }
  }
}

impl HasClient for Hastebin {
  fn client(&self) -> &Client {
    &self.client
  }
}

#[derive(Debug, Deserialize)]
struct HastebinSuccess {
  key: String
}

#[derive(Debug, Deserialize)]
struct HastebinError {
  error: String
}
