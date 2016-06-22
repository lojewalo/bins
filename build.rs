extern crate git2;
extern crate rustc_version;

use git2::{Repository, DescribeOptions, DescribeFormatOptions};
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::{self, Write};
use std::process::exit;
use rustc_version::version_matches;

fn get_version() -> String {
  let profile = env::var("PROFILE").unwrap();
  if profile == "release" {
    String::new()
  } else {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let repo = match Repository::open(&manifest_dir) {
      Ok(r) => r,
      Err(_) => return String::new()
    };
    let version = repo
    .describe(
      DescribeOptions::new().describe_tags().show_commit_oid_as_fallback(true)
    )
    .unwrap()
    .format(
      Some(DescribeFormatOptions::new().dirty_suffix("-dirty"))
    )
    .unwrap();
    String::from("-") + &version
  }
}

fn main() {
  if !version_matches(">= 1.8.0") {
    writeln!(&mut io::stderr(), "bins requires at least Rust 1.8.0").unwrap();
    exit(1);
  }
  let profile = env::var("PROFILE").unwrap();
  let version = get_version();
  let out_dir = env::var("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("git_short_tag.rs");
  let mut f = File::create(&dest_path).unwrap();
  f.write_all(format!("
      fn git_short_tag() -> &'static str {{
          \"{}\"
      }}
  ", version).as_bytes()).unwrap();
}
