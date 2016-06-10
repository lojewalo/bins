pub mod gist;
pub mod hastebin;
pub mod pastie;
pub mod pastebin;
mod batch;

use bins::PasteFile;
use bins::Bins;

pub trait Engine {
  fn upload(&self, bins: &Bins, data: &Vec<PasteFile>) -> Result<String, String>;
}
