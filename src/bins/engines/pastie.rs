use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use hyper::client::Response;
use bins::engines::batch::{BatchUpload, UploadsBatches, ProducesUrl, ProducesBody};
use hyper::header::{Headers, ContentType};
use url::form_urlencoded;

pub struct Pastie {
  batch_upload: BatchUpload
}

impl Pastie {
  pub fn new() -> Self {
    let mut headers = Headers::new();
    &headers.set(ContentType::form_url_encoded());
    Pastie {
      batch_upload: BatchUpload {
        url: String::from("http://pastie.org/pastes"),
        headers: headers,
        url_producer: Box::new(PastieUrlProducer { }),
        body_producer: Box::new(PastieBodyProducer { })
      }
    }
  }
}

struct PastieUrlProducer { }

impl ProducesUrl for PastieUrlProducer {
  #[allow(unused_variables)]
  fn produce_url(&self, bins: &Bins, res: Response, data: String) -> Result<String> {
    Ok(res.url.as_str().to_owned())
  }
}

struct PastieBodyProducer { }

impl ProducesBody for PastieBodyProducer {
  #[allow(unused_variables)]
  fn produce_body(&self, bins: &Bins, data: &PasteFile) -> Result<String> {
    Ok(
      form_urlencoded::Serializer::new(String::new())
        .append_pair("paste[body]", &data.data)
        .append_pair("paste[authorization]", "burger")
        .append_pair("paste[restricted]", if bins.arguments.private { "1" } else { "0" })
        .finish()
    )
  }
}

impl Engine for Pastie {
  fn upload(&self, bins: &Bins, data: &Vec<PasteFile>) -> Result<String> {
    self.batch_upload.upload(bins, data)
  }
}
