use bins::error::*;
use bins::{Bins, PasteFile};
use std::iter::repeat;
use hyper::client::Client;
use hyper::client::Response;
use hyper::header::Headers;
use hyper::Url;
use std::io::Read;
use hyper::status::StatusCode;
use std::collections::HashMap;

pub struct Index {
  pub file_urls: HashMap<String, String>
}

impl Index {
  pub fn from(string: &str) -> Result<Index> {
    let lines: Vec<&str> = string.split("\n").collect();
    if lines.len() < 4 {
      return Err(ErrorKind::InvalidIndexError.into());
    }
    let possible_urls: HashMap<Option<&str>, Option<&str>> = lines.iter()
      .skip(3)
      .filter(|s| !s.trim().is_empty())
      .map(|s| {
        let mut split = s.split(" ");
        let name = split.nth(1).map(|x| x[..x.len() - 1].as_ref());
        let url = split.nth(0);
        (name, url)
      })
      .collect();
    if possible_urls.iter().any(|t| t.0.is_none() || t.1.is_none()) {
      return Err(ErrorKind::InvalidIndexError.into());
    }
    let urls: HashMap<String, String> = possible_urls.iter()
      .map(|o| {
        (o.0.expect("none were none, but one was none").to_owned(),
         o.1.expect("none were none, but one was none").to_owned())
      })
      .collect();
    Ok(Index { file_urls: urls })
  }
}

pub struct IndexedUpload {
  pub url: String,
  pub headers: Headers,
  pub url_producer: Box<ProducesUrl>,
  pub body_producer: Box<ProducesBody>
}

pub trait ProducesUrl {
  fn produce_url(&self, bins: &Bins, res: Response, data: String) -> Result<String>;
}

pub trait ProducesBody {
  fn produce_body(&self, bins: &Bins, data: &PasteFile) -> Result<String>;
}

pub trait UploadsIndices {
  fn real_upload(&self, bins: &Bins, data: &PasteFile) -> Result<String>;

  fn upload(&self, bins: &Bins, data: &[PasteFile]) -> Result<String> {
    if data.len() < 2 {
      return self.real_upload(bins, &data[0]);
    }
    let wrapped_urls = data.iter()
      .map(|f| self.real_upload(bins, f))
      .map(|r| r.map_err(|e| e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n")))
      .collect::<Vec<_>>();
    for url in wrapped_urls.iter().cloned() {
      if url.is_err() {
        return Err(url.err().unwrap().into());
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
    let index_url = try!(self.real_upload(bins,
                                          &PasteFile {
                                            name: String::from("index"),
                                            data: index
                                          }));
    Ok(index_url)
  }

  fn generate_index(&self, data: &[PasteFile]) -> String {
    let header = format!("{} files", data.len());
    let separator = Self::repeat_str("-", header.len());
    let mut body = String::from("");
    for (i, file) in data.iter().enumerate() {
      let number = i + 1;
      body.push_str(&format!("{number}. {name}: <url{number}>\n",
                             number = number,
                             name = file.name));
    }
    header + "\n" + &separator + "\n\n" + &body
  }

  fn repeat_str(string: &str, count: usize) -> String {
    repeat(string).take(count).collect()
  }
}

impl UploadsIndices for IndexedUpload {
  fn real_upload(&self, bins: &Bins, data: &PasteFile) -> Result<String> {
    let client = Client::new();
    let mut res = try!(client.post(&self.url)
      .headers(self.headers.clone())
      .body(&try!(self.body_producer.as_ref().produce_body(bins, data)))
      .send()
      .map_err(|e| e.to_string()));
    let mut s = String::from("");
    try!(res.read_to_string(&mut s).map_err(|e| e.to_string()));
    // 404 for pastie, which appears to have issues when redirecting?
    if res.status != StatusCode::Ok && res.status != StatusCode::NotFound {
      println!("{}", s);
      return Err("paste could not be created".into());
    }
    self.url_producer.as_ref().produce_url(bins, res, s)
  }
}

pub struct IndexedDownload {
  pub url: String,
  pub headers: Headers,
  pub target: Option<String>
}

pub trait DownloadsFile {
  fn download(&self) -> Result<String>;
}

impl DownloadsFile for IndexedDownload {
  fn download(&self) -> Result<String> {
    let client = Client::new();
    let mut res = try!(client.get(&self.url).headers(self.headers.clone()).send());
    if res.status != StatusCode::Ok {
      return Err(format!("status was not ok: {}", res.status).into());
    }
    let mut s = String::from("");
    try!(res.read_to_string(&mut s));
    Ok(s)
  }
}

pub trait ChecksIndices {
  fn check_index(&self, bins: &Bins, downloaded: &String) -> Result<Url> {
    if let Ok(index) = Index::from(&downloaded) {
      let urls: HashMap<String, &String> = index.file_urls.iter().map(|(k, v)| (k.to_lowercase(), v)).collect();
      if urls.len() < 1 {
        return Err("index had no files".into());
      }
      let target_file = bins.arguments.files.get(0);
      if urls.len() > 1 && target_file.is_none() {
        let file_names = index.file_urls.iter().map(|(s, _)| String::from("  ") + s).collect::<Vec<_>>().join("\n");
        let message = format!("index had more than one file, but no target file was specified\n\nfiles available:\n{}",
                              file_names);
        return Err(message.into());
      }
      let target = target_file.unwrap_or_else(|| &urls.iter().next().expect("len() > 0, but no first element").0)
        .to_lowercase();
      if !urls.contains_key(&target) {
        return Err("index did not contain file".into());
      }
      match Url::parse(urls[&target].as_ref()) {
        Ok(u) => return Ok(u),
        Err(e) => return Err(e.to_string().into()),
      }
    }
    Err(ErrorKind::InvalidIndexError.into())
  }
}
