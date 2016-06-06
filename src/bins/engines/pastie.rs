use bins::PasteFile;
use bins::engines::Engine;
use config::types::Config;
use hyper::client::Response;
use bins::engines::batch::{BatchUpload, UploadsBatches, ProducesUrl, ProducesBody};
use hyper::header::Headers;
use url::form_urlencoded;

pub struct Pastie {
  batch_upload: BatchUpload
}

impl Pastie {
  pub fn new() -> Self {
    Pastie {
      batch_upload: BatchUpload {
        url: String::from("http://pastie.org/pastes"),
        headers: Headers::new(),
        url_producer: Box::new(PastieUrlProducer { }),
        body_producer: Box::new(PastieBodyProducer { })
      }
    }
  }
}

struct PastieUrlProducer { }

impl ProducesUrl for PastieUrlProducer {
  #[allow(unused_variables)]
  fn produce_url(&self, config: &Config, res: Response, data: String) -> Result<String, String> {
    Ok(res.url.as_str().to_owned())
  }
}

struct PastieBodyProducer { }

impl ProducesBody for PastieBodyProducer {
  #[allow(unused_variables)]
  fn produce_body(&self, config: &Config, data: &PasteFile) -> Result<String, String> {
    Ok(
      form_urlencoded::Serializer::new(String::new())
        .append_pair("paste[body]", &data.data)
        .append_pair("paste[authorization]", "burger")
        .append_pair("paste[restricted]", "1")
        .finish()
    )
  }
}

impl Engine for Pastie {
  fn upload(&self, config: &Config, data: &Vec<PasteFile>) -> Result<String, String> {
    self.batch_upload.upload(config, data)
  }
}
