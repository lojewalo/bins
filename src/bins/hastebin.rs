use url::Url;
use hyper::Client;
use serde_json;
use serde_json::Result as JsonResult;

use lib::*;
use lib::Result;
use lib::error::*;
use lib::files::*;

use std::io::Read;

// TODO: custom servers

pub struct Hastebin {
  client: Client
}

impl Hastebin {
  pub fn new() -> Hastebin {
    Hastebin {
      client: ::new_client()
    }
  }

  fn id_from_url(&self, url: &str) -> Option<String> {
    let url = option!(Url::parse(url).ok());
    let segments = option!(url.path_segments());
    let last_segment = option!(segments.last());
    last_segment.split('.').next().map(|x| x.to_owned())
  }
}

impl Bin for Hastebin {
  fn name(&self) -> &str {
    "hastebin"
  }

  fn html_host(&self) -> &str {
    "hastebin.com"
  }

  fn raw_host(&self) -> &str {
    "hastebin.com"
  }
}

impl ManagesUrls for Hastebin {}

impl CreatesUrls for Hastebin {}

impl FormatsUrls for Hastebin {}

impl FormatsHtmlUrls for Hastebin {
  fn format_html_url(&self, id: &str) -> Option<String> {
    Some(format!("https://hastebin.com/{}", id))
  }
}

impl FormatsRawUrls for Hastebin {
  fn format_raw_url(&self, id: &str) -> Option<String> {
    Some(format!("https://hastebin.com/raw/{}", id))
  }
}

impl CreatesHtmlUrls for Hastebin {
  fn create_html_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let html_url = self.format_html_url(id).unwrap();
    let raw_url = self.format_raw_url(id).unwrap();
    let mut res = self.client.get(&raw_url).send().map_err(ErrorKind::Http)?;
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(ErrorKind::Io)?;
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      return Err(ErrorKind::InvalidStatus(res.status_raw().0, Some(content)).into());
    }
    let parsed: serde_json::Result<Vec<IndexedFile>> = serde_json::from_str(&content);
    match parsed {
      Ok(is) => {
        debug!("file was an index, so checking its urls");
        let ids: Option<Vec<(String, String)>> = is.iter().map(|x| self.id_from_html_url(&x.url).map(|i| (x.name.clone(), i))).collect();
        let ids = match ids {
          Some(i) => i,
          None => {
            debug!("could not parse an ID from one of the URLs in the index");
            bail!("one of the URLs in the index did not contain a valid ID");
          }
        };
        Ok(ids.into_iter().map(|(name, id)| PasteUrl::raw(Some(PasteFileName::Explicit(name)), self.format_html_url(&id).unwrap())).collect())
      },
      Err(_) => Ok(vec![PasteUrl::Downloaded(html_url, DownloadedFile::new(PasteFileName::Guessed(id.to_owned()), content))])
    }
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl CreatesRawUrls for Hastebin {
  fn create_raw_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    debug!("creating raw url for {}", id);
    let raw_url = self.format_raw_url(id).unwrap();
    let mut res = self.client.get(&raw_url).send().map_err(ErrorKind::Http)?;
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(ErrorKind::Io)?;
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      return Err(ErrorKind::InvalidStatus(res.status_raw().0, Some(content)).into());
    }
    let parsed: serde_json::Result<Vec<IndexedFile>> = serde_json::from_str(&content);
    match parsed {
      Ok(is) => {
        debug!("file was an index, so checking its urls");
        let ids: Option<Vec<(String, String)>> = is.iter().map(|x| self.id_from_html_url(&x.url).map(|i| (x.name.clone(), i))).collect();
        let ids = match ids {
          Some(i) => i,
          None => {
            debug!("could not parse an ID from one of the URLs in the index");
            bail!("one of the URLs in the index did not contain a valid ID");
          }
        };
        Ok(ids.into_iter().map(|(name, id)| PasteUrl::raw(Some(PasteFileName::Explicit(name)), self.format_raw_url(&id).unwrap())).collect())
      },
      Err(_) => Ok(vec![PasteUrl::Downloaded(raw_url, DownloadedFile::new(PasteFileName::Guessed(id.to_owned()), content))])
    }
  }

  fn id_from_raw_url(&self, url: &str) -> Option<String> {
    self.id_from_url(url)
  }
}

impl HasFeatures for Hastebin {
  fn features(&self) -> Vec<BinFeature> {
    vec![BinFeature::Public, BinFeature::Anonymous]
  }
}

impl UploadsSingleFiles for Hastebin {
  fn upload_single(&self, file: &UploadFile) -> Result<PasteUrl> {
    debug!("uploading single file");
    let mut res = self.client.post("https://hastebin.com/documents")
      .body(&file.content)
      .send()
      .map_err(ErrorKind::Http)?;
    debug!("res: {:?}", res);
    let mut content = String::new();
    res.read_to_string(&mut content).map_err(ErrorKind::Io)?;
    debug!("content: {}", content);
    let success: JsonResult<HastebinSuccess> = serde_json::from_str(&content);
    debug!("success parse: {:?}", success);
    if let Ok(success) = success {
      debug!("upload was a success. creating html url");
      let url = self.format_html_url(&success.key).unwrap();
      return Ok(PasteUrl::html(Some(PasteFileName::Explicit(file.name.clone())), url));
    }
    debug!("parse was a failure, try to parse as error");
    let error: JsonResult<HastebinError> = serde_json::from_str(&content);
    debug!("error parse: {:?}", error);
    if let Ok(e) = error {
      return Err(ErrorKind::BinError(e.message).into());
    }
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      Err(ErrorKind::InvalidStatus(res.status_raw().0, Some(content)).into())
    } else {
      Err(ErrorKind::InvalidResponse.into())
    }
  }
}

impl HasClient for Hastebin {
  fn client(&self) -> &Client {
    &self.client
  }
}

#[derive(Debug, Deserialize)]
struct HastebinSuccess {
  key: String
}

#[derive(Debug, Deserialize)]
struct HastebinError {
  message: String
}
