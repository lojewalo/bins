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

lazy_static! {
  pub static ref ENGINES: Vec<Box<Engine>> = {
      vec![
        Box::new(gist::Gist::new()),
        Box::new(hastebin::Hastebin::new()),
        Box::new(pastie::Pastie::new()),
        Box::new(pastebin::Pastebin::new()),
        Box::new(sprunge::Sprunge::new())
      ]
  };
}

pub fn get_engine_names<'a>() -> Vec<&'a str> {
  ENGINES.iter().map(|e| e.get_name()).collect()
}

pub fn get_engine_by_name<'a>(name: &'a str) -> Option<&Box<Engine>> {
  ENGINES.iter().find(|e| e.get_name().to_lowercase() == name.to_lowercase())
}

pub fn get_engine_by_domain<'a>(domain: &'a str) -> Option<&Box<Engine>> {
  ENGINES.iter().find(|e| e.get_domain().to_lowercase() == domain.to_lowercase())
}

pub trait Engine: Sync {
  fn get_name(&self) -> &str;

  fn get_domain(&self) -> &str;

  fn upload(&self, bins: &Bins, data: &[PasteFile]) -> Result<String>;

  fn get_raw(&self, bins: &Bins, url: &mut Url) -> Result<String>;
}
