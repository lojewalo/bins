use bins::PasteFile;
use bins::engines::Engine;
use config::types::Config;
use std::iter::repeat;
use hyper::client::Client;
use std::io::Read;
use hyper::status::StatusCode;
use url::form_urlencoded;

pub struct Pastie;

impl Pastie {
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
    let mut res = try!(
      client.post("http://pastie.org/pastes")
        .body(&encoded)
        .send()
        .map_err(|e| e.to_string())
    );
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    if res.status != StatusCode::Ok {
      println!("{}", s);
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
