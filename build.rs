extern crate git2;

use git2::{Repository, DescribeOptions, DescribeFormatOptions};

use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Write;

fn main() {
  let profile = env::var("PROFILE").unwrap();
  if profile == "release" {
    return;
  }
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let repo = match Repository::open(&manifest_dir) {
    Ok(r) => r,
    Err(e) => panic!("Could not read git repository in {}: {}", manifest_dir, e)
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

  let out_dir = env::var("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("git_short_tag.rs");
  let mut f = File::create(&dest_path).unwrap();
  f.write_all(format!("
      fn git_short_tag() -> &'static str {{
          \"-{}\"
      }}
  ", version).as_bytes()).unwrap();
}
