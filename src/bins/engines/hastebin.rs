use bins::PasteFile;
use bins::engines::Engine;
use config::types::Config;
use hyper::client::Response;
use rustc_serialize::json::Json;
use bins::engines::batch::{BatchUpload, UploadsBatches, ProducesUrl, ProducesBody};
use hyper::header::Headers;

pub struct Hastebin {
  batch_upload: BatchUpload
}

impl Hastebin {
  pub fn new() -> Self {
    Hastebin {
      batch_upload: BatchUpload {
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
  fn produce_url(&self, config: &Config, res: Response, data: String) -> Result<String, String> {
    let raw_response = try!(Json::from_str(&data).map_err(|e| e.to_string()));
    let response = some_or_err!(raw_response.as_object(), String::from("response was not a json object"));
    let raw_key = some_or_err!(response.get("key"), String::from("no key"));
    let key = some_or_err!(raw_key.as_string(), String::from("key was not a string"));
    let scheme = res.url.scheme();
    let host = some_or_err!(res.url.host_str(), String::from("no host string"));
    Ok(format!("{}://{}/{}", scheme, host, key))
  }
}

struct HastebinBodyProducer { }

impl ProducesBody for HastebinBodyProducer {
  fn produce_body(&self, config: &Config, data: &PasteFile) -> Result<String, String> {
    Ok(data.clone().data)
  }
}

impl Engine for Hastebin {
  fn upload(&self, config: &Config, data: &Vec<PasteFile>) -> Result<String, String> {
    self.batch_upload.upload(config, data)
  }
}
