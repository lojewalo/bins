#![recursion_limit = "1024"]

extern crate url;
extern crate hyper;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;
#[macro_use]
extern crate log;
extern crate scoped_threadpool;
extern crate num_cpus;
#[cfg(feature = "file_type_checking")]
extern crate magic;
#[macro_use]
extern crate error_chain;
extern crate num_traits;

pub mod error;
pub mod files;
pub mod range;

use error::*;
use range::{BidirectionalRange, AnyContains};
use files::*;

type Client = hyper::Client<hyper::client::connect::HttpConnector>;

use scoped_threadpool::Pool;

use std::str::FromStr;
use std::io::Read;
use std::sync::mpsc::channel;
use std::collections::HashMap;

pub use error::Result;

pub trait Bin: Uploads + Downloads + ManagesUrls + HasFeatures {
  fn name(&self) -> &str;

  fn html_host(&self) -> &str;

  fn raw_host(&self) -> &str;
}

pub trait ManagesUrls: FormatsUrls + CreatesUrls {}

pub trait CreatesUrls: CreatesHtmlUrls + CreatesRawUrls {}

pub trait CreatesHtmlUrls {
  fn create_html_url(&self, id: &str) -> Result<Vec<PasteUrl>>;

  fn id_from_html_url(&self, url: &str) -> Option<String>;
}

pub trait CreatesRawUrls {
  fn create_raw_url(&self, id: &str) -> Result<Vec<PasteUrl>>;

  fn id_from_raw_url(&self, url: &str) -> Option<String>;
}

pub trait FormatsUrls: FormatsHtmlUrls + FormatsRawUrls {}

pub trait FormatsHtmlUrls {
  fn format_html_url(&self, id: &str) -> Option<String>;
}

pub trait FormatsRawUrls {
  fn format_raw_url(&self, id: &str) -> Option<String>;
}

pub trait HasFeatures {
  fn features(&self) -> Vec<BinFeature>;
}

pub trait Uploads {
  fn upload(&self, contents: &[UploadFile], index: bool) -> Result<Vec<PasteUrl>>;
}

pub trait UploadsSingleFiles {
  fn upload_single(&self, content: &UploadFile) -> Result<PasteUrl>;
}

pub trait Downloads {
  fn download(&self, id: &str, info: &DownloadInfo) -> Result<Paste>;
}

pub trait HasClient {
  fn client(&self) -> &Client;
}

impl<T> Uploads for T
  where T: UploadsSingleFiles + Sync
{
  fn upload(&self, contents: &[UploadFile], index: bool) -> Result<Vec<PasteUrl>> {
    if contents.len() == 1 {
      debug!("only one file to upload");
      return self.upload_single(&contents[0]).map(|x| vec![x]);
    }
    debug!("multiple files to upload");
    let (tx, rx) = channel();
    let mut pool = Pool::new(num_cpus::get() as u32);
    let channel_size = contents.len();
    let mut urls: Vec<IndexedFile> = Vec::with_capacity(channel_size);
    let res: Result<()> = pool.scoped(|scope| {
      for file in contents {
        let name = file.name.clone();
        let tx_clone = tx.clone();
        debug!("queuing scoped upload thread");
        scope.execute(move || {
          debug!("upload thread executing");
          if let Err(e) = tx_clone.send((name, self.upload_single(file))) {
            error!("could not send upload result over channel: {}", e);
          }
        });
      }
      debug!("joining on all threads");
      scope.join_all();
      debug!("done joining");
      for (name, result) in rx.into_iter().take(channel_size) {
        let upload = result?;
        urls.push(IndexedFile::new(name, upload.url().to_owned()));
      }
      debug!("sorting uploads");
      urls.sort_by_key(|i| i.name.clone());
      Ok(())
    });
    res?;
    if index {
      debug!("creating index");
      let index = serde_json::to_string_pretty(&urls)?;
      debug!("uploading index");
      self.upload_single(&UploadFile::new("index.json".to_owned(), index)).map(|x| vec![x])
    } else {
      Ok(urls.into_iter()
        .map(|indexed_file| {
          PasteUrl::Html {
            name: Some(PasteFileName::Explicit(indexed_file.name)),
            url: indexed_file.url
          }
        })
        .collect())
    }
  }
}

#[derive(Debug, Default)]
pub struct DownloadInfo {
  names: Option<Vec<String>>,
  range: Option<Vec<BidirectionalRange<usize>>>
}

impl DownloadInfo {
  pub fn names(names: &[&str]) -> DownloadInfo {
    let ns = names.iter().map(|x| x.to_string()).collect();
    DownloadInfo {
      names: Some(ns),
      ..DownloadInfo::default()
    }
  }

  pub fn range(range: &[BidirectionalRange<usize>]) -> DownloadInfo {
    DownloadInfo {
      range: Some(range.to_vec()),
      ..DownloadInfo::default()
    }
  }

  #[inline]
  pub fn empty() -> DownloadInfo {
    DownloadInfo::default()
  }
}

impl<T> Downloads for T
  where T: CreatesUrls + HasClient + Sync
{
  fn download(&self, id: &str, info: &DownloadInfo) -> Result<Paste> {
    debug!("downloading id {}", id);
    let raw_url_strs = self.create_raw_url(id)?;
    debug!("using raw urls {:?}", raw_url_strs);
    let (tx, rx) = channel();
    let mut pool = Pool::new(num_cpus::get() as u32);
    let channel_size = raw_url_strs.len();
    let mut contents = Vec::with_capacity(channel_size);
    pool.scoped(|scope| {
      for (i, url) in raw_url_strs.into_iter().enumerate() {
        let tx_clone = tx.clone();
        debug!("queuing scoped download thread");
        scope.execute(move || {
          if let Some(ref ns) = info.names {
            if let Some(PasteFileName::Explicit(name)) = url.name() {
              if !ns.contains(&name) {
                debug!("skipping {}", name);
                if let Err(e) = tx_clone.send(Ok(None)) {
                  error!("could not send result over channel: {}", e);
                }
                return;
              }
            }
          } else if let Some(ref range) = info.range {
            if !range.any_contains(i + 1) {
              debug!("skipping url {}", i);
              if let Err(e) = tx_clone.send(Ok(None)) {
                error!("could not send result over channel: {}", e);
              }
              return;
            }
          }
          if let PasteUrl::Downloaded(u, f) = url {
            debug!("already downloaded {}", u);
            if let Err(e) = tx_clone.send(Ok(Some((i, f)))) {
              error!("could not send result over channel: {}", e);
            }
            return;
          } else {
            debug!("downloading {:?}", url);
            // TODO: Actual async
            use futures::executor::block_on;
            let mut res = match block_on(self.client().get(hyper::Uri::from_str(url.url()).unwrap()))
              .map_err(ErrorKind::Http) {
              Ok(r) => r,
              Err(e) => {
                if let Err(tx_e) = tx_clone.send(Err(e)) {
                  error!("error sending result over channel: {}", tx_e);
                }
                return;
              }
            };
            let bytes = match block_on(hyper::body::to_bytes(res.body_mut())).map_err(ErrorKind::Http) {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Error reading bytes from body: {}", e);
                    unreachable!()
                }
            };
            let content = match String::from_utf8(Vec::from(&*bytes)) {
                Ok(content) => content,
                Err(e) => {
                    error!("Invalid UTF8 in response: {}", e);
                    unreachable!()
                }
            };
            if !res.status().is_success() {
              debug!("bad status code");
              let e = ErrorKind::InvalidStatus(res.status().as_u16(), Some(content));
              if let Err(tx_e) = tx_clone.send(Err(e)) {
                error!("error sending result over channel: {}", tx_e);
              }
              return;
            }
            let downloaded_file = DownloadedFile::new(
              url.name().unwrap_or_else(|| PasteFileName::Guessed(id.to_owned())),
              content
            );
            let tx_res = tx_clone.send(Ok(Some((i, downloaded_file))));
            if let Err(e) = tx_res {
              error!("could not send result over channel: {}", e);
            }
          }
        });
      }
      debug!("joining on all threads");
      scope.join_all();
      debug!("done joining");
      let mut map = HashMap::new();
      for result in rx.into_iter().take(channel_size) {
        let option = result?;
        if let Some((i, f)) = option {
          map.insert(i, f);
        }
      }
      debug!("sorting downloads");
      if let Some(ref range) = info.range {
        let order: Vec<usize> = range.iter().flat_map(|r| r.clone().collect::<Vec<_>>()).collect();
        for i in order {
          if i == 0 {
            // block against subtracting 1 from 0 on a usize
            bail!("range out of bounds: asked for item {af}, but there was no item {af}", af = i);
          }
          let item = match map.remove(&(i - 1)) {
            Some(x) => x,
            None => bail!("range out of bounds: asked for item {af}, but there was no item {af}", af = i)
          };
          contents.push(item);
        }
      } else {
        let keys: Vec<usize> = map.keys().cloned().collect();
        for key in keys {
          let file = match map.remove(&key) {
            Some(x) => x,
            None => return Err(ErrorKind::Other)
          };
          contents.push(file);
        }
        contents.sort_by_key(|f| f.name.name());
      }
      Ok(())
    })?;
    debug!("contents downloaded: {:?}", contents);
    if contents.is_empty() {
      debug!("no files downloaded. displaying filter error");
      bail!("filter returned no results");
    }
    let res = if contents.len() == 1 {
      debug!("only one file downloaded");
      let content = &contents[0];
      Paste::Single(content.clone())
    } else {
      debug!("multiple files downloaded");
      Paste::Multiple(contents)
    };
    debug!("as paste: {:?}", res);
    Ok(res)
  }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum BinFeature {
  Private,
  Public,
  Authed,
  Anonymous,
  MultiFile,
  SingleNaming
}

impl ::std::fmt::Display for BinFeature {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    let desc = match *self {
      BinFeature::Private => "private",
      BinFeature::Public => "public",
      BinFeature::Authed => "authed",
      BinFeature::Anonymous => "anonymous",
      BinFeature::MultiFile => "multiple-file",
      BinFeature::SingleNaming => "single-file named"
    };
    write!(f, "{}", desc)
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PasteUrl {
  Html {
    name: Option<PasteFileName>,
    url: String
  },
  Raw {
    name: Option<PasteFileName>,
    url: String
  },
  Downloaded(String, DownloadedFile)
}

impl PasteUrl {
  pub fn html(name: Option<PasteFileName>, url: String) -> PasteUrl {
    PasteUrl::Html {
      name: name,
      url: url
    }
  }

  pub fn raw(name: Option<PasteFileName>, url: String) -> PasteUrl {
    PasteUrl::Raw {
      name: name,
      url: url
    }
  }

  pub fn name(&self) -> Option<PasteFileName> {
    match *self {
      PasteUrl::Html { ref name, .. } |
      PasteUrl::Raw { ref name, .. } => name.clone(),
      PasteUrl::Downloaded(_, ref file) => Some(file.name.clone())
    }
  }

  pub fn url(&self) -> &str {
    match *self {
      PasteUrl::Html { ref url, .. } |
      PasteUrl::Raw { ref url, .. } |
      PasteUrl::Downloaded(ref url, _) => url
    }
  }
}
