use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use hyper::client::Response;
use bins::engines::indexed::{IndexedUpload, UploadsIndices, ProducesUrl, ProducesBody};
use hyper::header::Headers;
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
  #[allow(unused_variables)]
  fn produce_url(&self, bins: &Bins, res: Response, data: String) -> Result<String> {
    Ok(data)
  }
}

struct SprungeBodyProducer { }

impl ProducesBody for SprungeBodyProducer {
  #[allow(unused_variables)]
  fn produce_body(&self, bins: &Bins, data: &PasteFile) -> Result<String> {
    Ok(
      form_urlencoded::Serializer::new(String::new())
        .append_pair("sprunge", &data.data)
        .finish()
    )
  }
}

impl Engine for Sprunge {
  fn upload(&self, bins: &Bins, data: &Vec<PasteFile>) -> Result<String> {
    self.indexed_upload.upload(bins, data)
  }
}
