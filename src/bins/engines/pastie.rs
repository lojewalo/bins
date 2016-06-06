use bins::PasteFile;
use bins::engines::Engine;
use config::types::Config;
use hyper::client::Response;
use rustc_serialize::json::Json;
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
  fn produce_url(&self, config: &Config, res: Response, data: String) -> Result<String, String> {
    Ok(res.url.as_str().to_owned())
  }
}

struct PastieBodyProducer { }

impl ProducesBody for PastieBodyProducer {
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


/*impl Pastie {
  pub fn new() -> Self {
    Pastie { }
  }

  fn real_upload(&self, data: &PasteFile) -> Result<String, String> {
    let encoded: String = form_urlencoded::Serializer::new(String::new())
      .append_pair("paste[body]", &data.data)
      .append_pair("paste[authorization]", "burger")
      .append_pair("paste[restricted]", "1")
      .finish();
    let client = Client::new();
    let res = try!(
      client.post("http://pastie.org/pastes")
        .body(&encoded)
        .send()
        .map_err(|e| e.to_string())
    );
    if res.status != StatusCode::Ok {
      return Err(String::from("pastie could not be created"));
    }
    Ok(res.url.as_str().to_owned())
  }

  fn generate_index(&self, data: &Vec<PasteFile>) -> String {
    let header = format!("{} files", data.len());
    let separator = Self::repeat_str("-", header.len());
    let mut body = String::from("");
    for file in data.iter().enumerate() {
      let number = file.0 + 1;
      &body.push_str(&format!("{number}. {name}: <url{number}>\n", number = number, name = file.1.name));
    }
    header + "\n" + &separator + "\n\n" + &body
  }

  fn repeat_str(string: &str, count: usize) -> String {
    repeat(string).take(count).collect()
  }
}

impl Engine for Pastie {
  fn upload(&self, config: &Config, data: &Vec<PasteFile>) -> Result<String, String> {
    if data.len() < 2 {
      return self.real_upload(&data[0]);
    }
    let wrapped_urls = data.iter().map(|f| self.real_upload(f)).collect::<Vec<_>>();
    for url in wrapped_urls.iter().cloned() {
      if url.is_err() {
        return Err(url.err().unwrap().to_string());
      }
    }
    let urls = wrapped_urls.iter().cloned().map(|r| r.unwrap());
    let mut index = self.generate_index(data);
    let mut number = 1;
    for url in urls {
      let replace = String::from("<url") + &number.to_string() + ">";
      index = index.replace(&replace, url.as_ref());
      number += 1;
    }
    let index_url = try!(self.real_upload(&PasteFile { name: String::from("index"), data: index }));
    Ok(index_url)
  }
}
*/
