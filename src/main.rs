extern crate config;
extern crate argparse;
extern crate hyper;
extern crate rustc_serialize;
extern crate url;
#[macro_use]
extern crate error_chain;

mod bins;

use bins::error::*;
use bins::Bins;
use bins::arguments;
use bins::configuration::{BinsConfiguration, Configurable};
use bins::engines::Engine;

macro_rules! or_exit {
    ($expr: expr) => { match $expr { Ok(x) => x, Err(e) => { for err in e.iter() { println!("{}", err); } return 1; } } };
}

fn make_bins() -> Result<Bins> {
  let configuration = BinsConfiguration::new();
  let config = try!(configuration.parse_config());
  let arguments = arguments::get_arguments(&config);
  Ok(Bins::new(config, arguments))
}

fn inner() -> i32 {
  let bins = or_exit!(make_bins());
  let to_paste = or_exit!(bins.get_to_paste());
  let engine = or_exit!(bins.get_engine());
  let url = or_exit!(engine.upload(&bins, &to_paste));
  println!("{}", url);
  0
}

fn main() {
  let exit_code = inner();
  std::process::exit(exit_code);
}
