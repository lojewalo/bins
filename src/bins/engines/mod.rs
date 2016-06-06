pub mod gist;
pub mod hastebin;
pub mod pastie;

use config::types::Config;
use bins::PasteFile;

pub trait Engine {
  fn upload(&self, config: &Config, data: &Vec<PasteFile>) -> Result<String, String>;
}
