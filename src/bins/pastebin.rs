use url::Url;
use url::form_urlencoded;
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_openssl::OpensslClient;
use hyper::header::ContentType;
use serde_json;

use lib::*;
use lib::Result;
use lib::error::*;
use lib::files::*;
use config::{Config, CommandLineOptions};

use std::io::Read;
use std::sync::Arc;

pub struct Pastebin {
  config: Arc<Config>,
  cli: Arc<CommandLineOptions>,
  client: Client
}

impl Pastebin {
  pub fn new(config: Arc<Config>, cli: Arc<CommandLineOptions>) -> Pastebin {
    let ssl = OpensslClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);
    Pastebin {
      config: config,
      cli: cli,
      client: client
    }
  }

  fn _format_raw_url(&self, id: &str) -> String {
    format!("https://pastebin.com/raw/{}", id)
  }

  fn _format_html_url(&self, id: &str) -> String {
    format!("https://pastebin.com/{}", id)
  }

  fn id_from_url(&self, url: &str) -> Option<String> {
    let url = option!(Url::parse(url).ok());
    let segments = option!(url.path_segments());
    segments.last().map(|x| x.to_owned())
  }
}

impl Bin for Pastebin {
  fn name(&self) -> &str {
    "pastebin"
  }

  fn html_host(&self) -> &str {
    "pastebin.com"
  }

  fn raw_host(&self) -> &str {
    "pastebin.com"
  }
}

impl ManagesUrls for Pastebin {}

impl FormatsUrls for Pastebin {}

impl CreatesUrls for Pastebin {}

impl FormatsHtmlUrls for Pastebin {
  fn format_html_url(&self, id: &str) -> Option<String> {
    Some(self._format_html_url(id))
  }
}

impl FormatsRawUrls for Pastebin {
  fn format_raw_url(&self, id: &str) -> Option<String> {
    Some(self._format_html_url(id))
  }
}

impl CreatesHtmlUrls for Pastebin {
  fn create_html_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    self.create_raw_url(id)
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl CreatesRawUrls for Pastebin {
  fn create_raw_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let url = self._format_raw_url(id);
    let mut res = self.client.get(&url).send().map_err(BinsError::Http)?;
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      return Err(BinsError::InvalidStatus(res.status_raw().0, Some(content)));
    }
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
        Ok(ids.into_iter().map(|(name, id)| PasteUrl::raw(Some(PasteFileName::Explicit(name)), self._format_raw_url(&id))).collect())
      },
      Err(_) => Ok(vec![PasteUrl::Downloaded(url, DownloadedFile::new(PasteFileName::Guessed(id.to_owned()), content))])
    }
  }

  fn id_from_raw_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl HasFeatures for Pastebin {
  fn features(&self) -> Vec<BinFeature> {
    // TODO: use pastebin's crappy login system to allow authed pastes
    vec![BinFeature::Public,
         BinFeature::Private,
         BinFeature::Anonymous,
         BinFeature::SingleNaming]
  }
}

impl UploadsSingleFiles for Pastebin {
  fn upload_single(&self, contents: &UploadFile) -> Result<PasteUrl> {
    debug!(target: "pastebin", "uploading single file");
    let api_key = match self.config.pastebin.api_key {
      Some(ref key) if !key.is_empty() => key,
      _ => return Err(BinsError::Custom(String::from("no pastebin api key set")))
    };
    let mut res = self.client.post("https://pastebin.com/api/api_post.php")
      .body(&form_urlencoded::Serializer::new(String::new())
        .append_pair("api_option", "paste")
        .append_pair("api_paste_code", &contents.content)
        .append_pair("api_paste_private", self.cli.private.or(self.config.defaults.private).map(|x| if x { "1" } else { "0" }).unwrap_or("0"))
        .append_pair("api_paste_name", &contents.name)
        .append_pair("api_dev_key", api_key)
        .finish())
      .header(ContentType::form_url_encoded())
      .send()
      .map_err(BinsError::Http)?;
    debug!(target: "pastebin", "response: {:?}", res);
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    debug!(target: "pastebin", "content: {}", content);
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      return Err(BinsError::InvalidStatus(res.status_raw().0, Some(content)));
    }
    let url = content.replace("\n", "");
    Ok(PasteUrl::html(Some(PasteFileName::Explicit(contents.name.clone())), url))
  }
}

impl HasClient for Pastebin {
  fn client(&self) -> &Client {
    &self.client
  }
}
