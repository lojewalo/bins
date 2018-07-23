use url::Url;
use uuid::Uuid;
use hyper::Client;
use hyper::client::RequestBuilder;
use hyper::header::{Headers, ContentType, UserAgent, Authorization};
use serde_json;

use lib::*;
use lib::Result;
use lib::error::*;
use lib::files::*;
use config::{Config, CommandLineOptions};

use std::io::Read;
use std::sync::Arc;

pub struct PasteGg {
  config: Arc<Config>,
  cli: Arc<CommandLineOptions>,
  client: Client
}

impl PasteGg {
  pub fn new(config: Arc<Config>, cli: Arc<CommandLineOptions>) -> PasteGg {
    PasteGg {
      config: config,
      cli: cli,
      client: ::new_client()
    }
  }

  fn add_headers<'a>(&self, rb: RequestBuilder<'a>) -> RequestBuilder<'a> {
    let mut headers = Headers::new();
    headers.set(ContentType::json());
    headers.set(UserAgent(format!("bins/{}", crate_version!())));
    if let Some(true) = self.cli.authed.or(self.config.defaults.authed) {
      if let Some(ref key) = self.config.pastegg.key {
        if !key.is_empty() {
          headers.set(Authorization(format!("Key {}", key)));
        }
      }
    }
    rb.headers(headers)
  }

  fn get_paste(&self, id: &str) -> Result<PasteGgPaste<FullPasteGgFile>> {
    debug!("getting paste for ID {}", id);
    let builder = self.client.get(&format!("https://api.paste.gg/v0/pastes/{}?full=true", id));
    let mut res = self.add_headers(builder).send()?;
    let mut content = String::new();
    res.read_to_string(&mut content)?;
    if res.status.class().default_code() != ::hyper::Ok {
      debug!("bad status code");
      return Err(ErrorKind::InvalidStatus(res.status_raw().0, Some(content)).into());
    }
    let result: PasteGgResult<PasteGgPaste<FullPasteGgFile>> = serde_json::from_str(&content)
      .chain_err(|| "could not parse paste.gg response")?;
    match result {
      PasteGgResult::Success { result } => Ok(result),
      PasteGgResult::Error { error, message } => {
        let mut msg = error;
        if let Some(m) = message {
          msg += ": ";
          msg += &m;
        }
        Err(ErrorKind::BinError(msg).into())
      },
    }
  }
}

impl Bin for PasteGg {
  fn name(&self) -> &str {
    "pastegg"
  }

  fn html_host(&self) -> &str {
    "paste.gg"
  }

  fn raw_host(&self) -> &str {
    "paste.gg"
  }
}

impl ManagesUrls for PasteGg {}

impl CreatesUrls for PasteGg {}

impl FormatsUrls for PasteGg {}

impl FormatsHtmlUrls for PasteGg {
  fn format_html_url(&self, _: &str) -> Option<String> {
    None
  }
}

impl FormatsRawUrls for PasteGg {
  fn format_raw_url(&self, _: &str) -> Option<String> {
    None
  }
}

impl CreatesHtmlUrls for PasteGg {
  fn create_html_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let paste = self.get_paste(id)?;
    let urls: Vec<PasteUrl> = paste.files.iter()
      .map(|file| PasteUrl::html(
        file.name.clone().map(PasteFileName::Explicit),
        format!("https://paste.gg/{}", id)
      ))
      .collect();
    Ok(urls)
  }

  fn id_from_html_url(&self, url: &str) -> Option<String> {
    self.id_from_raw_url(url)
  }
}

impl CreatesRawUrls for PasteGg {
  fn create_raw_url(&self, id: &str) -> Result<Vec<PasteUrl>> {
    let paste = self.get_paste(id)?;
    let urls: Vec<PasteUrl> = paste.files.iter()
      .map(|file| PasteUrl::raw(
        file.name.clone().map(PasteFileName::Explicit),
        format!("https://api.paste.gg/v0/pastes/{}/files/{}/raw", paste.id, file.id)
      ))
      .collect();
    Ok(urls)
  }

  fn id_from_raw_url(&self, url: &str) -> Option<String> {
    let mut url = option!(Url::parse(url).ok());
    url.set_fragment(None);
    url.set_query(None);
    option!(url.path_segments())
      .flat_map(|x| Uuid::parse_str(x))
      .map(|x| x.simple().to_string())
      .next()
  }
}

impl HasFeatures for PasteGg {
  fn features(&self) -> Vec<BinFeature> {
    vec![
      BinFeature::Public,
      BinFeature::Private,
      BinFeature::Authed,
      BinFeature::Anonymous,
      BinFeature::MultiFile,
      BinFeature::SingleNaming,
    ]
  }
}

impl Uploads for PasteGg {
  fn upload(&self, contents: &[UploadFile], _: bool) -> Result<Vec<PasteUrl>> {
    let files: Vec<PasteGgUploadFile> = contents
      .iter()
      .map(|file| PasteGgUploadFile {
        name: Some(file.name.clone()),
        content: PasteGgContent::Text(file.content.clone()),
      })
      .collect();
    let visibility = if self.cli.private.or(self.config.defaults.private).map(|x| !x).unwrap_or(false) {
      Visibility::Public
    } else {
      Visibility::Unlisted
    };
    let upload_file = PasteGgUpload {
      name: None,
      description: None,
      visibility,
      files,
    };
    let upload_json = serde_json::to_string(&upload_file)?;
    let builder = self.client.post("https://api.paste.gg/v0/pastes").body(&upload_json);
    let mut res = self.add_headers(builder).send()?;
    let mut content = String::new();
    res.read_to_string(&mut content)?;
    // if res.status != ::hyper::status::StatusCode::Created {
    //   return Err(ErrorKind::BinError(content).into());
    // }
    let paste: PasteGgResult<PasteGgPaste<PartialPasteGgFile>> = serde_json::from_str(&content)?;
    let paste = match paste {
      PasteGgResult::Success { result } => result,
      PasteGgResult::Error { error, message } => {
        let mut msg = error;
        if let Some(m) = message {
          msg += ": ";
          msg += &m;
        }
        return Err(ErrorKind::BinError(msg).into())
      },
    };
    Ok(vec![PasteUrl::html(None, format!("https://paste.gg/{}", paste.id))])
  }
}

impl HasClient for PasteGg {
  fn client(&self) -> &Client {
    &self.client
  }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum PasteGgResult<T> {
  Success {
    result: T
  },
  Error {
    error: String,
    message: Option<String>,
  },
}

#[derive(Debug, Deserialize)]
struct PasteGgPaste<T> {
  id: String,
  name: Option<String>,
  description: Option<String>,
  visibility: Visibility,
  files: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct FullPasteGgFile {
  id: String,
  name: Option<String>,
  content: PasteGgContent,
}

#[derive(Debug, Deserialize)]
struct PartialPasteGgFile {
  id: String,
  name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "format", content = "value")]
enum PasteGgContent {
  Text(String),
  Base64(String),
  Gzip(String),
  Xz(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Visibility {
  Public,
  Unlisted,
  Private,
}

#[derive(Debug, Serialize)]
struct PasteGgUpload {
  name: Option<String>,
  description: Option<String>,
  visibility: Visibility,
  files: Vec<PasteGgUploadFile>,
}

#[derive(Debug, Serialize)]
struct PasteGgUploadFile {
  name: Option<String>,
  content: PasteGgContent,
}
