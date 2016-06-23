use bins::error::*;
use bins::configuration::BetterLookups;
use bins::engines::{Bin, ConvertUrlsToRawUrls, ProduceRawContent, UploadContent, UsesIndices, VerifyUrl};
use bins::network::download::{Downloader, ModifyDownloadRequest};
use bins::network::upload::{ModifyUploadRequest, Uploader};
use bins::network::{self, RequestModifiers};
use bins::{Bins, PasteFile};
use hyper::header::{ContentType, Headers, Referer};
use hyper::Url;
use std::cell::RefCell;
use url::form_urlencoded;

pub struct Pastebin {
  // totally thread safe
  last_url: RefCell<Option<String>>
}

impl Pastebin {
  pub fn new() -> Self {
    Pastebin { last_url: RefCell::new(None) }
  }
}

impl Bin for Pastebin {
  fn get_name(&self) -> &str {
    "pastebin"
  }

  fn get_domain(&self) -> &str {
    "pastebin.com"
  }
}

impl VerifyUrl for Pastebin {
  fn verify_url(&self, url: &Url) -> bool {
    self.segments(url).len() == 1
  }
}

impl UploadContent for Pastebin {
  fn upload_paste(&self, bins: &Bins, content: PasteFile) -> Result<Url> {
    let url = try!(network::parse_url("http://pastebin.com/api/api_post.php"));
    let mut response = try!(self.upload(&url, bins, &content));
    network::parse_url(try!(network::read_response(&mut response)))
  }
}

impl ConvertUrlsToRawUrls for Pastebin {
  fn convert_url_to_raw_url(&self, _: &Bins, url: &Url) -> Result<Url> {
    *self.last_url.borrow_mut() = Some(url.as_str().to_owned());
    let mut url = url.clone();
    let new_path = {
      String::from("/download") + url.path()
    };
    url.set_path(&new_path);
    Ok(url)
  }
}

impl ModifyUploadRequest for Pastebin {
  fn modify_request<'a>(&'a self, bins: &Bins, content: &PasteFile) -> Result<RequestModifiers> {
    let api_key = some_or_err!(bins.config.lookup_str("pastebin.api_key"),
                               "no pastebin.api_key defined in configuration file".into());
    if api_key.is_empty() {
      return Err("pastebin.api_key was empty".into());
    }
    let body = form_urlencoded::Serializer::new(String::new())
      .append_pair("api_option", "paste")
      .append_pair("api_dev_key", api_key)
      .append_pair("api_paste_private",
                   if bins.arguments.private {
                     "1"
                   } else {
                     "0"
                   })
      .append_pair("api_paste_code", &content.data)
      .append_pair("api_paste_name", &content.name)
      .finish();
    let mut headers = Headers::new();
    headers.set(ContentType::form_url_encoded());
    Ok(RequestModifiers {
      body: Some(body),
      headers: Some(headers)
    })
  }
}

unsafe impl Sync for Pastebin {}

impl UsesIndices for Pastebin {}

impl ProduceRawContent for Pastebin {}

impl Uploader for Pastebin {}

impl Downloader for Pastebin {}

impl ModifyDownloadRequest for Pastebin {
  fn modify_request(&self, _: &Bins) -> Result<RequestModifiers> {
    let mut headers = Headers::new();
    let url = self.last_url.borrow_mut();
    if url.is_some() {
      let url = some_ref_or_err!(url, "no referer (this is a bug)".into());
      headers.set(Referer(url.clone()));
    }
    Ok(RequestModifiers { headers: Some(headers), ..RequestModifiers::default() })
  }
}
