use bins::PasteFile;
use bins::engines::Engine;
use config::types::Config;
use std::iter::repeat;
use hyper::client::Client;
use std::io::Read;
use hyper::status::StatusCode;
use hyper::header::ContentType;
use url::form_urlencoded;

pub struct Pastebin;

impl Pastebin {
  pub fn new() -> Self {
    Pastebin { }
  }

  fn real_upload(&self, config: &Config, data: &PasteFile) -> Result<String, String> {
    let api_key = some_or_err!(config.lookup_str("pastebin.api_key"), String::from("no pastebin.api_key defined in configuration file"));
    if api_key.is_empty() {
      return Err(String::from("no pastebin.api_key defined"));
    }
    let encoded: String = form_urlencoded::Serializer::new(String::new())
      .append_pair("api_option", "paste")
      .append_pair("api_dev_key", &api_key)
      .append_pair("api_paste_code", &data.data)
      .append_pair("api_paste_name", &data.name)
      // .append_pair("api_paste_private", "1") // max 25 for free accounts
      .finish();
    let client = Client::new();
    let mut res = try!(
      client.post("http://pastebin.com/api/api_post.php")
        .header(ContentType::form_url_encoded())
        .body(&encoded)
        .send()
        .map_err(|e| e.to_string())
    );
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    if res.status != StatusCode::Ok {
      println!("{}", s);
      return Err(String::from("Pastebin could not be created"));
    }
    Ok(s)
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

impl Engine for Pastebin {
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
}
