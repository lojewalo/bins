use bins::error::*;
use bins::engines::{Bin, ConvertUrlsToRawUrls, ProduceRawContent, UploadUrl, UsesIndices, VerifyUrl};
use bins::network::download::{Downloader, ModifyDownloadRequest};
use bins::network::RequestModifiers;
use bins::network::upload::{ModifyUploadRequest, Uploader};
use bins::{Bins, PasteFile};
use hyper::Url;
use url::form_urlencoded;

pub struct Sprunge;

impl Sprunge {
  pub fn new() -> Self {
    Sprunge {}
  }
}

impl Bin for Sprunge {
  fn get_name(&self) -> &str {
    "sprunge"
  }

  fn get_domain(&self) -> &str {
    "sprunge.us"
  }
}

impl VerifyUrl for Sprunge {
  fn verify_url(&self, url: &Url) -> bool {
    self.segments(url).len() == 1
  }
}

impl UploadUrl for Sprunge {
  fn get_upload_url(&self) -> &str {
    "http://sprunge.us/"
  }
}

impl ConvertUrlsToRawUrls for Sprunge {
  fn convert_url_to_raw_url(&self, _: &Bins, url: &Url) -> Result<Url> {
    let mut u = url.clone();
    u.set_query(None);
    Ok(u)
  }
}

impl ModifyUploadRequest for Sprunge {
  fn modify_request<'a>(&'a self, _: &Bins, content: &PasteFile) -> Result<RequestModifiers> {
    let body = form_urlencoded::Serializer::new(String::new())
      .append_pair("sprunge", &content.data)
      .finish();
    Ok(RequestModifiers { body: Some(body), ..RequestModifiers::default() })
  }
}

unsafe impl Sync for Sprunge {}

impl UsesIndices for Sprunge {}

impl ProduceRawContent for Sprunge {}

impl Uploader for Sprunge {}

impl Downloader for Sprunge {}

impl ModifyDownloadRequest for Sprunge {}
