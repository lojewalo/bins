use bins::PasteFile;
use bins::engines::Engine;
use config::types::Config;
use hyper::client::Response;
use bins::engines::batch::{BatchUpload, UploadsBatches, ProducesUrl, ProducesBody};
use hyper::header::{Headers, ContentType};
use url::form_urlencoded;

pub struct Pastebin {
  batch_upload: BatchUpload
}

impl Pastebin {
  pub fn new() -> Self {
    let mut headers = Headers::new();
    &headers.set(ContentType::form_url_encoded());
    Pastebin {
      batch_upload: BatchUpload {
        url: String::from("http://pastebin.com/api/api_post.php"),
        headers: headers,
        url_producer: Box::new(PastebinUrlProducer { }),
        body_producer: Box::new(PastebinBodyProducer { })
      }
    }
  }
}

struct PastebinUrlProducer { }

impl ProducesUrl for PastebinUrlProducer {
  fn produce_url(&self, config: &Config, res: Response, data: String) -> Result<String, String> {
    Ok(data)
  }
}

struct PastebinBodyProducer { }

impl ProducesBody for PastebinBodyProducer {
  fn produce_body(&self, config: &Config, data: &PasteFile) -> Result<String, String> {
    let api_key = some_or_err!(config.lookup_str("pastebin.api_key"), String::from("no pastebin.api_key defined in configuration file"));
    if api_key.is_empty() {
      return Err(String::from("no pastebin.api_key defined"));
    }
    Ok(
      form_urlencoded::Serializer::new(String::new())
        .append_pair("api_option", "paste")
        .append_pair("api_dev_key", &api_key)
        .append_pair("api_paste_code", &data.data)
        .append_pair("api_paste_name", &data.name)
        .finish()
    )
  }
}

impl Engine for Pastebin {
  fn upload(&self, config: &Config, data: &Vec<PasteFile>) -> Result<String, String> {
    self.batch_upload.upload(config, data)
  }
}
