use bins::error::*;
use bins::Bins;
use bins::configuration::BinsConfiguration;
use bins::engines;
use bins::FlexibleRange;
use bins::network;
use clap::{App, Arg, ArgGroup};
use hyper::Url;
use std::path::Path;
use std::process;

pub struct Arguments {
  pub all: bool,
  pub auth: bool,
  pub bin: Option<String>,
  pub copy: bool,
  pub files: Vec<String>,
  pub force: bool,
  pub input: Option<String>,
  pub json: bool,
  pub message: Option<String>,
  pub name: Option<String>,
  pub number_lines: bool,
  pub output: Option<String>,
  pub private: bool,
  pub range: Option<FlexibleRange>,
  pub raw_urls: bool,
  pub server: Option<Url>,
  pub urls: bool,
  pub write: bool
}

include!(concat!(env!("OUT_DIR"), "/git_short_tag.rs"));

fn get_name() -> String {
  option_env!("CARGO_PKG_NAME").unwrap_or("unknown_name").to_owned()
}

fn get_version() -> String {
  let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown_version").to_owned();
  let git_tag = git_short_tag();
  format!("{}{}", version, git_tag)
}

cfg_if! {
  if #[cfg(feature = "clipboard_support")] {
    fn get_clipboard_args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
      vec![Arg::with_name("copy")
             .short("C")
             .long("copy")
             .help("copies the output of the command to the clipboard without a newline")
             .conflicts_with("no-copy"),
           Arg::with_name("no-copy")
             .short("c")
             .long("no-copy")
             .help("does not copy the output of the command to the clipboard")]
    }
  } else {
    fn get_clipboard_args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
      Vec::new()
    }
  }
}

pub fn get_arguments(config: &BinsConfiguration) -> Result<Arguments> {
  let mut arguments = Arguments {
    all: false,
    auth: config.get_defaults_auth(),
    bin: config.get_defaults_bin().map(|s| s.to_owned()),
    copy: config.get_defaults_copy(),
    files: Vec::new(),
    force: false,
    input: None,
    json: false,
    message: None,
    name: None,
    number_lines: false,
    output: None,
    private: config.get_defaults_private(),
    range: None,
    raw_urls: false,
    server: None,
    urls: false,
    write: false
  };
  let name = get_name();
  let version = get_version();
  let mut app = App::new(name.as_ref())
    .version(version.as_ref())
    .about("A tool for pasting from the terminal")
    .arg(Arg::with_name("files")
      .help("files to paste")
      .takes_value(true)
      .multiple(true))
    .arg(Arg::with_name("message")
      .short("m")
      .long("message")
      .help("message to paste")
      .use_delimiter(false)
      .takes_value(true)
      .value_name("string"))
    .arg(Arg::with_name("private")
      .short("p")
      .long("private")
      .help("if the paste should be private")
      .conflicts_with("public"))
    .arg(Arg::with_name("public")
      .short("P")
      .long("public")
      .help("if the paste should be public"))
    .arg(Arg::with_name("auth")
      .short("a")
      .long("auth")
      .help("if authentication (like api keys and tokens) should be used")
      .conflicts_with("anon"))
    .arg(Arg::with_name("anon")
      .short("A")
      .long("anon")
      .help("if pastes should be posted without authentication"))
    .arg(Arg::with_name("bin")
      .short("b")
      .long("bin")
      .help("bin to use when uploading")
      .takes_value(true)
      .possible_values(&*engines::get_bin_names()))
    .arg(Arg::with_name("service")
      .short("s")
      .long("service")
      .help("legacy flag included for backwards compatibility. use --bin, as this will be removed in 2.0.0")
      .takes_value(true)
      .possible_values(&*engines::get_bin_names()))
    .group(ArgGroup::with_name("bin_or_service")
      .args(&["bin", "service"])
      .required(arguments.bin.is_none()))
    .arg(Arg::with_name("list-bins")
      .short("l")
      .long("list-bins")
      .help("lists available bins and exits")
      .conflicts_with_all(&["files", "message", "private", "public", "auth", "anon", "bin_or_service", "input"]))
    .arg(Arg::with_name("list-services")
      .long("list-services")
      .help("legacy flag included for backwards compatibility. use --list-bins, as this will be removed in 2.0.0")
      .conflicts_with_all(&["files", "message", "private", "public", "auth", "anon", "bin_or_service", "input"]))
    .group(ArgGroup::with_name("list-bins_or_list-services").args(&["list-bins", "list-services"]))
    .arg(Arg::with_name("input")
      .short("i")
      .long("input")
      .help("displays raw contents of input paste")
      .takes_value(true)
      .value_name("url")
      .conflicts_with_all(&["auth", "anon", "public", "private", "message", "bin_or_service"]))
    .arg(Arg::with_name("range")
      .short("n")
      .long("range")
      .help("chooses the files to get in input mode, starting from 0 (e.g. \"0\", \"0,1\", \"0-2\", \"2-0,3\")")
      .takes_value(true)
      .value_name("range")
      .use_delimiter(false)
      .requires("input")
      .conflicts_with("files"))
    .arg(Arg::with_name("all")
      .short("L")
      .long("all")
      .help("gets all files in input mode")
      .requires("input")
      .conflicts_with_all(&["files", "range"]))
    .arg(Arg::with_name("raw-urls")
      .short("r")
      .long("raw-urls")
      .help("gets the raw urls instead of the content in input mode")
      .requires("input"))
    .arg(Arg::with_name("urls")
      .short("u")
      .long("urls")
      .help("gets the urls instead of the content in input mode")
      .requires("input")
      .conflicts_with("raw-urls"))
    .arg(Arg::with_name("server")
      .short("S")
      .long("server")
      .help("specifies the server to use for the service (only support on hastebin)")
      .takes_value(true)
      .value_name("server_url"))
    .arg(Arg::with_name("name")
      .short("N")
      .long("name")
      .help("specifies a file name for --message or stdin")
      .takes_value(true)
      .value_name("name")
      .conflicts_with("files"))
    .arg(Arg::with_name("force")
      .short("f")
      .long("force")
      .help("overrides warnings about file type or size when uploading")
      .conflicts_with("input"))
    .arg(Arg::with_name("number_lines")
      .short("e")
      .long("number-lines")
      .help("display line numbers for each file in input mode")
      .requires("input"))
    .arg(Arg::with_name("write")
      .short("w")
      .long("write")
      .help("writes pastes to files in input mode")
      .requires("input"))
    .arg(Arg::with_name("output")
      .short("o")
      .long("output")
      .help("specifies where to save files in write mode")
      .takes_value(true)
      .value_name("dir")
      .requires("write"))
    .arg(Arg::with_name("json")
      .short("j")
      .long("json")
      .help("output json a object instead of normal values")
      .conflicts_with_all(&["write", "urls", "raw-urls"]));
  for arg in get_clipboard_args() {
    app = app.arg(arg);
  }
  let res = app.get_matches();
  if res.is_present("list-bins_or_list-services") {
    println!("{}", engines::get_bin_names().join("\n"));
    process::exit(0);
  }
  if let Some(files) = res.values_of("files") {
    arguments.files = files.map(|s| s.to_owned()).collect();
  }
  if let Some(message) = res.value_of("message") {
    arguments.message = Some(message.to_owned());
  }
  if let Some(bin) = res.value_of("bin_or_service") {
    arguments.bin = Some(bin.to_owned());
  }
  if let Some(input) = res.value_of("input") {
    arguments.input = Some(input.to_owned());
  }
  if let Some(range) = res.value_of("range") {
    arguments.range = Some(try!(FlexibleRange::parse(range)));
  }
  if let Some(server) = res.value_of("server") {
    if let Some(ref bin) = arguments.bin {
      if bin.to_lowercase() != "hastebin" {
        return Err("--server may only be used if --service is hastebin".into());
      }
    }
    arguments.server = Some(try!(network::parse_url(server).chain_err(|| "invalid --server")));
  }
  if let Some(name) = res.value_of("name") {
    let name = try!(Bins::sanitize_path(Path::new(name)));
    arguments.name = Some(name.to_owned());
  }
  if let Some(output) = res.value_of("output") {
    arguments.output = Some(output.to_owned());
  }
  arguments.all = res.is_present("all");
  arguments.force = res.is_present("force");
  arguments.json = res.is_present("json");
  arguments.number_lines = res.is_present("number_lines");
  arguments.raw_urls = res.is_present("raw-urls");
  arguments.urls = res.is_present("urls");
  arguments.write = res.is_present("write");
  if res.is_present("private") {
    arguments.private = true;
  } else if res.is_present("public") {
    arguments.private = false;
  }
  if res.is_present("anon") {
    arguments.auth = false;
  } else if res.is_present("auth") {
    arguments.auth = true;
  }
  if res.is_present("copy") {
    arguments.copy = true;
  } else if res.is_present("no-copy") {
    arguments.copy = false;
  }
  Ok(arguments)
}
