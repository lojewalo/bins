use bins::error::*;
use bins::{Bins, PasteFile};
use bins::engines::Engine;
use hyper::client::Response;
use bins::engines::indexed::{IndexedUpload, UploadsIndices, ProducesUrl, ProducesBody};
use bins::engines::indexed::{ChecksIndices, IndexedDownload, DownloadsFile};
use hyper::header::{Headers, ContentType};
use hyper::Url;
use url::form_urlencoded;

pub struct Pastie {
  indexed_upload: IndexedUpload
}

unsafe impl Sync for Pastie {}

impl Pastie {
  pub fn new() -> Self {
    let mut headers = Headers::new();
    headers.set(ContentType::form_url_encoded());
    Pastie {
      indexed_upload: IndexedUpload {
        url: String::from("http://pastie.org/pastes"),
        headers: headers,
        url_producer: Box::new(PastieUrlProducer {}),
        body_producer: Box::new(PastieBodyProducer {})
      }
    }
  }
}

struct PastieUrlProducer { }

impl ProducesUrl for PastieUrlProducer {
  fn produce_url(&self, _: &Bins, res: Response, _: String) -> Result<String> {
    Ok(res.url.as_str().to_owned())
  }
}

struct PastieBodyProducer { }

impl ProducesBody for PastieBodyProducer {
  fn produce_body(&self, bins: &Bins, data: &PasteFile) -> Result<String> {
    Ok(form_urlencoded::Serializer::new(String::new())
      .append_pair("paste[body]", &data.data)
      .append_pair("paste[authorization]", "burger")
      .append_pair("paste[restricted]",
                   if bins.arguments.private {
                     "1"
                   } else {
                     "0"
                   })
      .finish())
  }
}

impl ChecksIndices for Pastie {}

impl Engine for Pastie {
  fn get_name(&self) -> &str {
    "pastie"
  }

  fn get_domain(&self) -> &str {
    "pastie.org"
  }

  fn upload(&self, bins: &Bins, data: &[PasteFile]) -> Result<String> {
    self.indexed_upload.upload(bins, data)
  }

  fn get_raw(&self, bins: &Bins, url: &mut Url) -> Result<String> {
    let new_path = {
      let path = url.path();
      if path.starts_with("/private") {
        return Err("pastie private pastes are not supported in input mode".into());
      }
      let path_segments = some_or_err!(url.path_segments(), "could not get path for url".into());
      if path_segments.count() > 1 {
        format!("{}/download", path)
      } else {
        format!("/pastes{}/download", path)
      }
    };
    url.set_path(&new_path);
    let download = IndexedDownload {
      url: String::from(url.as_str()),
      headers: Headers::new(),
      target: None
    };
    let downloaded = try!(download.download());
    match self.check_index(bins, &downloaded) {
      Ok(mut new_url) => return self.get_raw(bins, &mut new_url),
      Err(e) => {
        if let ErrorKind::InvalidIndexError = *e.kind() {} else {
          return Err(e);
        }
      }
    }
    Ok(downloaded)
  }
}
