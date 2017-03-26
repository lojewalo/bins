#[derive(Debug, Serialize)]
pub struct Error {
  pub message: String,
  pub causes: Vec<String>
}

impl Error {
  pub fn new<S: AsRef<str>>(message: S, causes: Vec<S>) -> Self {
    Error {
      message: message.as_ref().to_string(),
      causes: causes.into_iter().map(|s| s.as_ref().to_string()).collect()
    }
  }
}
