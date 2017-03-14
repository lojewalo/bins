use url::Url;
use url::form_urlencoded;
use hyper::Client;

use lib::{Bin, BinFeature, ManagesUrls, ManagesHtmlUrls, ManagesRawUrls, UploadsSingleFiles, HasClient, HasFeatures, PasteUrl};
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

impl ManagesHtmlUrls for Sprunge {
  fn create_html_url(&self, id: &str, _: &[&str]) -> Result<Vec<PasteUrl>> {
    Ok(vec![PasteUrl::html(None, self.create_url(id))])
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl ManagesRawUrls for Sprunge {
  fn create_raw_url(&self, id: &str, _: &[&str]) -> Result<Vec<PasteUrl>> {
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
  fn upload_single(&self, contents: &UploadFile) -> Result<String> {
    debug!(target: "sprunge", "uploading single file");
    let mut res = self.client.post("http://sprunge.us")
      .body(&form_urlencoded::Serializer::new(String::new())
        .append_pair("sprunge", &contents.content)
        .finish())
      .send()
      .map_err(BinsError::Http)?;
    debug!(target: "sprunge", "response: {:?}", res);
    if res.status.class().default_code() != ::hyper::Ok {
      return Err(BinsError::Http(::hyper::Error::Status));
    }
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(BinsError::Io)?;
    debug!(target: "sprunge", "content: {}", content);
    Ok(content.replace("\n", ""))
  }
}

impl HasClient for Sprunge {
  fn client(&self) -> &Client {
    &self.client
  }
}
