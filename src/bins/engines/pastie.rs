use bins::engines::{Bin, ConvertUrlsToRawUrls, ProduceRawContent, UploadContent, UsesIndices};
use bins::error::*;
use bins::network::download::{Downloader, ModifyDownloadRequest};
use bins::network::upload::{ModifyUploadRequest, Uploader};
use bins::network::{self, RequestModifiers};
use bins::{Bins, PasteFile};
use hyper::Url;
use url::form_urlencoded;
use hyper::header::{Headers, ContentType};

pub struct Pastie;

impl Pastie {
  pub fn new() -> Self {
    Pastie {}
  }
}

impl Bin for Pastie {
  fn get_name(&self) -> &str {
    "pastie"
  }

  fn get_domain(&self) -> &str {
    "pastie.org"
  }
}

impl UploadContent for Pastie {
  fn upload_paste(&self, bins: &Bins, content: PasteFile) -> Result<Url> {
    let url = try!(network::parse_url("http://pastie.org/pastes"));
    let response = try!(self.upload(&url, bins, &content));
    Ok(response.url.clone())
  }
}

impl ConvertUrlsToRawUrls for Pastie {
  fn convert_url_to_raw_url(&self, url: &Url) -> Result<Url> {
    let mut url = url.clone();
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
    Ok(url)
  }
}

impl ModifyUploadRequest for Pastie {
  fn modify_request<'a>(&'a self, bins: &Bins, content: &PasteFile) -> Result<RequestModifiers> {
    let body = form_urlencoded::Serializer::new(String::new())
      .append_pair("paste[body]", &content.data)
      .append_pair("paste[authorization]", "burger")
      .append_pair("paste[restricted]",
                   if bins.arguments.private {
                     "1"
                   } else {
                     "0"
                   })
      .finish();
    let mut headers = Headers::new();
    headers.set(ContentType::form_url_encoded());
    Ok(RequestModifiers {
      body: Some(body),
      headers: Some(headers),
      .. RequestModifiers::default()
    })
  }
}

unsafe impl Sync for Pastie {}

impl UsesIndices for Pastie {}

impl ProduceRawContent for Pastie {}

impl Uploader for Pastie {}

impl Downloader for Pastie {}

impl ModifyDownloadRequest for Pastie {}
