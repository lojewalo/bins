extern crate config;
extern crate clap;
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
use std::io::Write;

macro_rules! println_stderr {
  ($fmt:expr) => { { writeln!(std::io::stderr(), $fmt).expect("error writing to stderr"); } };
  ($fmt:expr, $($arg:tt)*) => { { writeln!(std::io::stderr(), $fmt, $($arg)*).expect("error writing to stderr"); } };
}

macro_rules! or_exit {
  ($expr: expr) => { match $expr { Ok(x) => x, Err(e) => { for err in e.iter() { println_stderr!("{}", err); } return 1; } } };
}

fn make_bins() -> Result<Bins> {
  let configuration = BinsConfiguration::new();
  let config = try!(configuration.parse_config());
  let arguments = arguments::get_arguments(&config);
  Ok(Bins::new(config, arguments))
}

fn inner() -> i32 {
  let bins = or_exit!(make_bins());
  let output = or_exit!(bins.get_output());
  println!("{}", output);
  0
}

fn main() {
  let exit_code = inner();
  std::process::exit(exit_code);
}
