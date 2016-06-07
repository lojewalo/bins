use config::types::Config;
use bins::PasteFile;
use std::iter::repeat;
use hyper::client::Client;
use hyper::client::Response;
use hyper::header::Headers;
use std::io::Read;
use hyper::status::StatusCode;

pub struct BatchUpload {
  pub url: String,
  pub headers: Headers,
  pub url_producer: Box<ProducesUrl>,
  pub body_producer: Box<ProducesBody>
}

pub trait ProducesUrl {
  fn produce_url(&self, config: &Config, res: Response, data: String) -> Result<String, String>;
}

pub trait ProducesBody {
  fn produce_body(&self, config: &Config, data: &PasteFile) -> Result<String, String>;
}

pub trait UploadsBatches {
  fn real_upload(&self, config: &Config, data: &PasteFile) -> Result<String, String>;

  fn upload(&self, config: &Config, data: &Vec<PasteFile>) -> Result<String, String> {
    if data.len() < 2 {
      return self.real_upload(config, &data[0]);
    }
    let wrapped_urls = data.iter().map(|f| self.real_upload(config, f)).collect::<Vec<_>>();
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
    let index_url = try!(self.real_upload(config, &PasteFile { name: String::from("index"), data: index }));
    Ok(index_url)
  }

  fn generate_index(&self, data: &Vec<PasteFile>) -> String {
    let header = format!("{} files", data.len());
    let separator = Self::repeat_str("-", header.len());
    let mut body = String::from("");
    for (i, file) in data.iter().enumerate() {
      let number = i + 1;
      &body.push_str(&format!("{number}. {name}: <url{number}>\n", number = number, name = file.name));
    }
    header + "\n" + &separator + "\n\n" + &body
  }

  fn repeat_str(string: &str, count: usize) -> String {
    repeat(string).take(count).collect()
  }
}

impl UploadsBatches for BatchUpload {
  fn real_upload(&self, config: &Config, data: &PasteFile) -> Result<String, String> {
    let client = Client::new();
    let mut res = try!(
      client.post(&self.url)
        .headers(self.headers.clone())
        .body(&try!(self.body_producer.as_ref().produce_body(config, data)))
        .send()
        .map_err(|e| e.to_string())
    );
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    if res.status != StatusCode::Ok {
      println!("{}", s);
      return Err(String::from("paste could not be created"));
    }
    self.url_producer.as_ref().produce_url(config, res, s)
  }
}
