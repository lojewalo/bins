use url::Url;
use url::form_urlencoded;
use hyper::Client;

use lib::*;
use lib::Result;
use lib::error::*;
use lib::files::*;
use config::{Config, CommandLineOptions};

use std::io::Read;
use std::sync::Arc;

pub struct Sprunge {
  config: Arc<Config>,
  cli: Arc<CommandLineOptions>,
  client: Client
}

impl Sprunge {
  pub fn new(config: Arc<Config>, cli: Arc<CommandLineOptions>) -> Sprunge {
    Sprunge {
      config: config,
      cli: cli,
      client: Client::new()
    }
  }

  fn create_url(&self, id: &str) -> String {
    // sprunge has no HTTPS support
    format!("http://sprunge.us/{}", id)
  }

  fn id_from_url(&self, url: &str) -> Option<String> {
    let mut url = option!(Url::parse(url).ok());
    url.set_query(None);
    let segments = option!(url.path_segments());
    segments.last().map(|x| x.to_owned())
  }
}

impl Bin for Sprunge {
  fn name(&self) -> &str {
    "sprunge"
  }

  fn html_host(&self) -> &str {
    "sprunge.us"
  }

  fn raw_host(&self) -> &str {
    "sprunge.us"
  }
}

impl ManagesUrls for Sprunge {}

impl FormatsUrls for Sprunge {}

impl CreatesUrls for Sprunge {}

impl FormatsHtmlUrls for Sprunge {
  fn format_html_url(&self, id: &str) -> Option<String> {
    Some(self.create_url(id))
  }
}

impl FormatsRawUrls for Sprunge {
  fn format_raw_url(&self, id: &str) -> Option<String> {
    Some(self.create_url(id))
  }
}

impl CreatesHtmlUrls for Sprunge {
  fn create_html_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    Ok(vec![PasteUrl::html(None, self.create_url(id))])
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl CreatesRawUrls for Sprunge {
  fn create_raw_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    Ok(vec![PasteUrl::raw(None, self.create_url(id))])
  }

  fn id_from_raw_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl HasFeatures for Sprunge {
  fn features(&self) -> Vec<BinFeature> {
    vec![BinFeature::Public, BinFeature::Anonymous]
  }
}

impl UploadsSingleFiles for Sprunge {
  fn upload_single(&self, contents: &UploadFile) -> Result<PasteUrl> {
    debug!(target: "sprunge", "uploading single file");
    let mut res = self.client.post("http://sprunge.us")
      .body(&form_urlencoded::Serializer::new(String::new())
        .append_pair("sprunge", &contents.content)
        .finish())
      .send()
      .map_err(BinsError::Http)?;
    debug!(target: "sprunge", "response: {:?}", res);
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    debug!(target: "sprunge", "content: {}", content);
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      return Err(BinsError::InvalidStatus(res.status_raw().0, Some(content)));
    }
    let url = content.replace("\n", "");
    Ok(PasteUrl::raw(Some(DownloadedFileName::Explicit(contents.name.clone())), url))
  }
}

impl HasClient for Sprunge {
  fn client(&self) -> &Client {
    &self.client
  }
}
