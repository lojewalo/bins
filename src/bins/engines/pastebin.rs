use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use hyper::client::Response;
use bins::engines::indexed::{IndexedUpload, UploadsIndices, ProducesUrl, ProducesBody};
use hyper::header::{Headers, ContentType};
use url::form_urlencoded;

pub struct Pastebin {
  indexed_upload: IndexedUpload
}

impl Pastebin {
  pub fn new() -> Self {
    let mut headers = Headers::new();
    headers.set(ContentType::form_url_encoded());
    Pastebin {
      indexed_upload: IndexedUpload {
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
  fn produce_url(&self, _: &Bins, _: Response, data: String) -> Result<String> {
    Ok(data)
  }
}

struct PastebinBodyProducer { }

impl ProducesBody for PastebinBodyProducer {
  fn produce_body(&self, bins: &Bins, data: &PasteFile) -> Result<String> {
    let api_key = some_or_err!(bins.config.lookup_str("pastebin.api_key"), "no pastebin.api_key defined in configuration file".into());
    if api_key.is_empty() {
      return Err("pastebin.api_key was empty".into());
    }
    Ok(
      form_urlencoded::Serializer::new(String::new())
        .append_pair("api_option", "paste")
        .append_pair("api_dev_key", api_key)
        .append_pair("api_paste_private", if bins.arguments.private { "1" } else { "0" })
        .append_pair("api_paste_code", &data.data)
        .append_pair("api_paste_name", &data.name)
        .finish()
    )
  }
}

impl Engine for Pastebin {
  fn upload(&self, bins: &Bins, data: &[PasteFile]) -> Result<String> {
    self.indexed_upload.upload(bins, data)
  }
}
