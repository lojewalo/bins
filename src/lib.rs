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

pub mod error;
pub mod files;

use error::BinsError;
use files::*;

use hyper::Client;

use scoped_threadpool::Pool;

use std::io::Read;
use std::sync::mpsc::channel;

pub use error::Result;

pub trait Bin: Uploads + Downloads + ManagesUrls + HasFeatures {
  fn name(&self) -> &str;

  fn html_host(&self) -> &str;

  fn raw_host(&self) -> &str;
}

pub trait ManagesUrls: ManagesHtmlUrls + ManagesRawUrls {}

pub trait ManagesHtmlUrls {
  fn create_html_url(&self, id: &str, names: &[&str]) -> Result<Vec<PasteUrl>>;

  fn id_from_html_url(&self, url: &str) -> Option<String>;
}

pub trait ManagesRawUrls {
  fn create_raw_url(&self, id: &str, names: &[&str]) -> Result<Vec<PasteUrl>>;

  fn id_from_raw_url(&self, url: &str) -> Option<String>;
}

pub trait HasFeatures {
  fn features(&self) -> Vec<BinFeature>;
}

pub trait Uploads {
  fn upload(&self, contents: &[UploadFile]) -> Result<String>;
}

pub trait UploadsSingleFiles {
  fn upload_single(&self, content: &UploadFile) -> Result<String>;
}

pub trait Downloads {
  fn download(&self, id: &str, names: Option<&[&str]>) -> Result<Paste>;
}

pub trait HasClient {
  fn client(&self) -> &Client;
}

impl<T> Uploads for T
  where T: UploadsSingleFiles + Sync
{
  fn upload(&self, contents: &[UploadFile]) -> Result<String> {
    if contents.len() == 1 {
      debug!("only one file to upload");
      return self.upload_single(&contents[0]);
    }
    debug!("multiple files to upload");
    let (tx, rx) = channel();
    let mut pool = Pool::new(num_cpus::get() as u32);
    let channel_size = contents.len();
    let mut urls: Vec<IndexedFile> = Vec::with_capacity(channel_size);
    pool.scoped(|scope| {
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
        urls.push(IndexedFile::new(name, upload));
      }
      Ok(())
    })?;
    debug!("creating index");
    let index = serde_json::to_string_pretty(&urls).map_err(BinsError::Json)?;
    debug!("uploading index");
    self.upload_single(&UploadFile::new("index.json".to_owned(), index))
  }
}

impl<T> Downloads for T
  where T: ManagesUrls + HasClient + Sync
{
  fn download(&self, id: &str, names: Option<&[&str]>) -> Result<Paste> {
    debug!("downloading id {}", id);
    let raw_url_strs = self.create_raw_url(id, names.unwrap_or_default())?;
    debug!("using raw urls {:?}", raw_url_strs);
    let (tx, rx) = channel();
    let mut pool = Pool::new(num_cpus::get() as u32);
    let channel_size = raw_url_strs.len();
    let mut contents = Vec::with_capacity(channel_size);
    pool.scoped(|scope| {
      for url in raw_url_strs {
        let tx_clone = tx.clone();
        debug!("queuing scoped download thread");
        scope.execute(move || {
          if let Some(ns) = names {
            if let Some(DownloadedFileName::Explicit(name)) = url.name() {
              if !ns.contains(&name.as_str()) {
                debug!("skipping {}", name);
                if let Err(e) = tx_clone.send(Ok(None)) {
                  error!("could not send result over channel: {}", e);
                }
                return;
              }
            }
          }
          if let PasteUrl::Downloaded(u, f) = url {
            debug!("already downloaded {}", u);
            if let Err(e) = tx_clone.send(Ok(Some(f))) {
              error!("could not send result over channel: {}", e);
            }
            return;
          } else {
            debug!("downloading {:?}", url);
            let mut res = match self.client().get(url.url())
              .send()
              .map_err(BinsError::Http) {
              Ok(r) => r,
              Err(e) => {
                if let Err(tx_e) = tx_clone.send(Err(e)) {
                  error!("error sending result over channel: {}", tx_e);
                }
                return;
              }
            };
            let mut content = String::new();
            if let Err(e) = res.read_to_string(&mut content).map_err(BinsError::Io) {
              if let Err(tx_e) = tx_clone.send(Err(e)) {
                error!("error sending result over channel: {}", tx_e);
              }
              return;
            }
            let tx_res = tx_clone.send(Ok(Some(DownloadedFile::new(
              url.name().unwrap_or_else(|| DownloadedFileName::Guessed(id.to_owned())),
              content))));
            if let Err(e) = tx_res {
              error!("could not send result over channel: {}", e);
            }
          }
        });
      }
      debug!("joining on all threads");
      scope.join_all();
      debug!("done joining");
      for result in rx.into_iter().take(channel_size) {
        let option = result?;
        if let Some(f) = option {
          contents.push(f);
        }
      }
      contents.sort_by_key(|f| f.name.name());
      Ok(())
    })?;
    debug!("contents downloaded: {:?}", contents);
    let res = if contents.len() == 1 {
      debug!("only one file downloaded");
      let content = &contents[0];
      Paste::Single(content.clone())
    } else {
      debug!("multiple files downloaded");
      Paste::MultiDownloaded(contents)
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
  MultiFile
}

impl ::std::fmt::Display for BinFeature {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    ::std::fmt::Debug::fmt(self, f)
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PasteUrl {
  Html {
    name: Option<DownloadedFileName>,
    url: String
  },
  Raw {
    name: Option<DownloadedFileName>,
    url: String
  },
  Downloaded(String, DownloadedFile)
}

impl PasteUrl {
  pub fn html(name: Option<DownloadedFileName>, url: String) -> PasteUrl {
    PasteUrl::Html {
      name: name,
      url: url
    }
  }

  pub fn raw(name: Option<DownloadedFileName>, url: String) -> PasteUrl {
    PasteUrl::Raw {
      name: name,
      url: url
    }
  }

  pub fn name(&self) -> Option<DownloadedFileName> {
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
