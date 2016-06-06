use bins::PasteFile;
use bins::engines::Engine;
use config::types::Config;
use std::iter::repeat;
use hyper::client::Client;
use std::io::Read;
use hyper::status::StatusCode;
use rustc_serialize::json::Json;

pub struct Hastebin;

impl Hastebin {
  pub fn new() -> Self {
    Hastebin { }
  }

  fn real_upload(&self, data: &PasteFile) -> Result<String, String> {
    let client = Client::new();
    let mut res = try!(
      client.post("http://hastebin.com/documents")
        .body(&data.data)
        .send()
        .map_err(|e| e.to_string())
    );
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    if res.status != StatusCode::Ok {
      println!("{}", s);
      return Err(String::from("hastebin could not be created"));
    }
    let raw_response = try!(Json::from_str(&s).map_err(|e| e.to_string()));
    let response = some_or_err!(raw_response.as_object(), String::from("response was not a json object"));
    let raw_key = some_or_err!(response.get("key"), String::from("no key"));
    let key = some_or_err!(raw_key.as_string(), String::from("key was not a string"));
    let scheme = res.url.scheme();
    let host = some_or_err!(res.url.host_str(), String::from("no host string"));
    Ok(format!("{}://{}/{}", scheme, host, key))
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

struct HastebinData {
  data: String
}

impl<'a> From<&'a Vec<PasteFile>> for HastebinData {
  fn from(files: &'a Vec<PasteFile>) -> Self {
    let data = if files.len() < 2 {
      files[0].clone().data
    } else {
      files.iter().map(|f| {
          let file = f.clone();
          format!(
            "{}\n{}\n\n{}",
            file.name,
            repeat("-").take(file.name.len()).collect::<String>(),
            file.data
          )
        }
      ).collect::<Vec<_>>().join("\n")
    };
    HastebinData { data: data }
  }
}

impl Engine for Hastebin {
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
