extern crate config;
extern crate argparse;
extern crate hyper;
extern crate rustc_serialize;
extern crate url;

mod bins;

use bins::Bins;
use bins::arguments;
use bins::configuration::{BinsConfiguration, Configurable};
use bins::engines::Engine;

macro_rules! or_exit {
    ($expr: expr) => { match $expr { Ok(x) => x, Err(e) => { println!("{}", e); return 1; } } };
}

fn make_bins() -> Result<Bins, String> {
  let arguments = arguments::get_arguments();
  let configuration = BinsConfiguration::new();
  let config = try!(configuration.parse_config().map_err(|e| format!("config error: {}", e)));
  Ok(Bins::new(config, arguments))
}

fn inner() -> i32 {
  let bins = or_exit!(make_bins());
  let to_paste = or_exit!(bins.get_to_paste());
  let engine = or_exit!(bins.get_engine());
  let url = or_exit!(engine.upload(&bins.config, &to_paste));
  println!("{}", url);
  0
}

fn main() {
  let exit_code = inner();
  std::process::exit(exit_code);
}
