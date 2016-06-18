use bins::error::*;
use bins::network::RequestModifiers;
use bins::{Bins, PasteFile};
use hyper::client::{Client, Response};
use hyper::header::Headers;
use hyper::Url;

pub trait Uploader: ModifyUploadRequest {
  fn upload(&self, url: &Url, bins: &Bins, content: &PasteFile) -> Result<Response> {
    let modifiers = try!(self.modify_request(bins, content));
    let body = modifiers.body.unwrap_or("".to_owned());
    let body = body.as_bytes();
    let headers = modifiers.headers.unwrap_or_else(Headers::new);
    let client = Client::new();
    let builder = client.post(url.as_str())
      .body(body)
      .headers(headers);
    Ok(try!(builder.send()))
  }
}

pub trait ModifyUploadRequest {
  fn modify_request<'a>(&'a self, _: &Bins, _: &PasteFile) -> Result<RequestModifiers> {
    Ok(RequestModifiers::default())
  }
}
