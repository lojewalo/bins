#[derive(Debug, Serialize, Deserialize)]
pub struct IndexedFile {
  pub name: String,
  pub url: String
}

impl IndexedFile {
  pub fn new(name: String, url: String) -> IndexedFile {
    IndexedFile {
      name: name,
      url: url
    }
  }
}

#[derive(Debug)]
pub struct UploadFile {
  pub name: String,
  pub content: String
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Paste {
  Single(DownloadedFile),
  Multiple(Vec<DownloadedFile>),
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadedFile {
  pub name: PasteFileName,
  pub content: String
}

impl DownloadedFile {
  pub fn new(name: PasteFileName, content: String) -> DownloadedFile {
    DownloadedFile {
      name: name,
      content: content
    }
  }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PasteFileName {
  Explicit(String),
  Guessed(String)
}

impl PasteFileName {
  pub fn name(&self) -> String {
    match *self {
      PasteFileName::Explicit(ref name) |
      PasteFileName::Guessed(ref name) => name.clone()
    }
  }
}

impl UploadFile {
  pub fn new(name: String, content: String) -> UploadFile {
    UploadFile {
      name: name,
      content: content
    }
  }
}
