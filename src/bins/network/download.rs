use bins::error::*;
use hyper::client::{Client, Response};
use hyper::header::Headers;
use hyper::Url;
use bins::network::RequestModifiers;

pub trait Downloader: ModifyDownloadRequest {
  fn download(&self, url: &Url) -> Result<Response> {
    let modifiers = try!(self.modify_request());
    let body = modifiers.body.unwrap_or("".to_owned());
    let body = body.as_bytes();
    let headers = modifiers.headers.unwrap_or(Headers::new());
    let client = Client::new();
    Ok(try!(client.get(url.as_str()).body(body).headers(headers).send()))
  }
}

pub trait ModifyDownloadRequest {
  fn modify_request<'a>(&'a self) -> Result<RequestModifiers> {
    Ok(RequestModifiers::default())
  }
}
