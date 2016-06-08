use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use hyper::client::Response;
use rustc_serialize::json::Json;
use bins::engines::indexed::{IndexedUpload, UploadsIndices, ProducesUrl, ProducesBody};
use hyper::header::Headers;

pub struct Hastebin {
  indexed_upload: IndexedUpload
}

impl Hastebin {
  pub fn new() -> Self {
    Hastebin {
      indexed_upload: IndexedUpload {
        url: String::from("http://hastebin.com/documents"),
        headers: Headers::new(),
        url_producer: Box::new(HastebinUrlProducer { }),
        body_producer: Box::new(HastebinBodyProducer { })
      }
    }
  }
}

struct HastebinUrlProducer { }

impl ProducesUrl for HastebinUrlProducer {
  #[allow(unused_variables)]
  fn produce_url(&self, bins: &Bins, res: Response, data: String) -> Result<String> {
    let raw_response = try!(Json::from_str(&data).map_err(|e| e.to_string()));
    let response = some_or_err!(raw_response.as_object(), "response was not a json object".into());
    let raw_key = some_or_err!(response.get("key"), "no key".into());
    let key = some_or_err!(raw_key.as_string(), "key was not a string".into());
    let scheme = res.url.scheme();
    let host = some_or_err!(res.url.host_str(), "no host string".into());
    Ok(format!("{}://{}/{}", scheme, host, key))
  }
}

struct HastebinBodyProducer { }

impl ProducesBody for HastebinBodyProducer {
  #[allow(unused_variables)]
  fn produce_body(&self, bins: &Bins, data: &PasteFile) -> Result<String> {
    Ok(data.clone().data)
  }
}

impl Engine for Hastebin {
  fn upload(&self, bins: &Bins, data: &Vec<PasteFile>) -> Result<String> {
    self.indexed_upload.upload(bins, data)
  }
}
