extern crate git2;
extern crate rustc_version;
extern crate time;
#[macro_use]
extern crate clap;

use git2::{DescribeFormatOptions, DescribeOptions, Repository};
use rustc_version::Version;
use clap::Shell;

use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::exit;

include!("src/cli.rs");

fn completions() {
  let outdir = match env::var_os("OUT_DIR") {
    None => return,
    Some(outdir) => outdir,
  };
  let mut app = create_app(false);
  app.gen_completions("bins", Shell::Bash, &outdir);
  app.gen_completions("bins", Shell::Zsh, &outdir);
  app.gen_completions("bins", Shell::Fish, &outdir);
}

fn get_version<'a>() -> (String, Option<String>, String) {
  let profile = env::var("PROFILE").unwrap();
  let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
  let git = if let Ok(repo) = Repository::open(&manifest_dir) {
    Some(repo.describe(DescribeOptions::new().describe_tags().show_commit_oid_as_fallback(true))
      .unwrap()
      .format(Some(DescribeFormatOptions::new().dirty_suffix("-dirty")))
      .unwrap())
  } else {
    None
  };
  let date = format!("{}", time::now().strftime("%b %d, %Y %H:%M:%S %z").unwrap());
  (profile, git, date)
}

fn main() {
  if rustc_version::version().unwrap() < Version::parse("1.17.0-nightly").unwrap() {
    writeln!(&mut io::stderr(), "bins requires at least Rust 1.17.0").unwrap();
    exit(1);
  }
  let (profile, git, date) = get_version();
  let out_dir = env::var("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("version_info.rs");
  let mut f = File::create(&dest_path).unwrap();
  f.write_all(format!("
      struct VersionInfo {{
        profile: &'static str,
        git: Option<&'static str>,
        date: &'static str
      }}

      impl VersionInfo {{
        fn get() -> VersionInfo {{
            VersionInfo {{
              profile: \"{}\",
              git: {},
              date: \"{}\"
            }}
        }}
      }}
  ",
      profile,
      if let Some(ref g) = git { format!("Some(\"{}\")", g) } else { "None".to_owned() },
      date)
      .as_bytes())
    .unwrap();

    completions();
}
