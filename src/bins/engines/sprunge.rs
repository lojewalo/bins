use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use bins::engines::indexed::{IndexedUpload, UploadsIndices, ProducesUrl, ProducesBody};
use bins::engines::indexed::{IndexedDownload, DownloadsFile};
use hyper::client::Response;
use hyper::header::Headers;
use hyper::Url;
use url::form_urlencoded;

pub struct Sprunge {
  indexed_upload: IndexedUpload
}

impl Sprunge {
  pub fn new() -> Self {
    Sprunge {
      indexed_upload: IndexedUpload {
        url: String::from("http://sprunge.us"),
        headers: Headers::new(),
        url_producer: Box::new(SprungeUrlProducer { }),
        body_producer: Box::new(SprungeBodyProducer { })
      }
    }
  }
}

struct SprungeUrlProducer { }

impl ProducesUrl for SprungeUrlProducer {
  fn produce_url(&self, _: &Bins, _: Response, data: String) -> Result<String> {
    Ok(data)
  }
}

struct SprungeBodyProducer { }

impl ProducesBody for SprungeBodyProducer {
  fn produce_body(&self, _: &Bins, data: &PasteFile) -> Result<String> {
    Ok(
      form_urlencoded::Serializer::new(String::new())
        .append_pair("sprunge", &data.data)
        .finish()
    )
  }
}

impl Engine for Sprunge {
  fn upload(&self, bins: &Bins, data: &[PasteFile]) -> Result<String> {
    self.indexed_upload.upload(bins, data)
  }

  fn get_raw(&self, _: &Bins, url: &mut Url) -> Result<String> {
    // Remove language specification to get raw text
    url.set_query(None);
    let download = IndexedDownload {
      url: String::from(url.as_str()),
      headers: Headers::new(),
      target: None
    };
    download.download()
  }
}
