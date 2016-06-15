pub mod gist;
pub mod hastebin;
pub mod pastie;
pub mod pastebin;
pub mod sprunge;
mod indexed;

use bins::error::*;
use bins::PasteFile;
use bins::Bins;
use hyper::Url;

pub trait Engine {
  fn get_name(&self) -> &str;

  fn get_domain(&self) -> &str;

  fn upload(&self, bins: &Bins, data: &[PasteFile]) -> Result<String>;

  fn get_raw(&self, bins: &Bins, url: &mut Url) -> Result<String>;
}
