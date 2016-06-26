use bins::error::*;
use bins::configuration::BinsConfiguration;
use bins::engines;
use bins::FlexibleRange;
use bins::network;
use clap::{App, Arg};
use hyper::Url;
use std::path::{Component, Path};
use std::process;

pub struct Arguments {
  pub files: Vec<String>,
  pub message: Option<String>,
  pub service: Option<String>,
  pub private: bool,
  pub auth: bool,
  pub copy: bool,
  pub input: Option<String>,
  pub range: Option<FlexibleRange>,
  pub raw_urls: bool,
  pub urls: bool,
  pub all: bool,
  pub server: Option<Url>,
  pub name: Option<String>,
  pub force: bool,
  pub number_lines: bool
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
    files: Vec::new(),
    message: None,
    service: config.get_defaults_service().map(|s| s.to_owned()),
    private: config.get_defaults_private(),
    auth: config.get_defaults_auth(),
    copy: config.get_defaults_copy(),
    input: None,
    range: None,
    raw_urls: false,
    urls: false,
    all: false,
    server: None,
    name: None,
    force: false,
    number_lines: false
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
    .arg(Arg::with_name("service")
      .short("s")
      .long("service")
      .help("pastebin service to use")
      .takes_value(true)
      .possible_values(&*engines::get_bin_names())
      .required(arguments.service.is_none()))
    .arg(Arg::with_name("list-services")
      .short("l")
      .long("list-services")
      .help("lists available bins and exits")
      .conflicts_with_all(&["files", "message", "private", "public", "auth", "anon", "service", "input"]))
    .arg(Arg::with_name("input")
      .short("i")
      .long("input")
      .help("displays raw contents of input paste")
      .takes_value(true)
      .value_name("url")
      .conflicts_with_all(&["auth", "anon", "public", "private", "message", "service"]))
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
      .requires("input"));
  for arg in get_clipboard_args() {
    app = app.arg(arg);
  }
  let res = app.get_matches();
  if res.is_present("list-services") {
    println!("{}", engines::get_bin_names().join("\n"));
    process::exit(0);
  }
  if let Some(files) = res.values_of("files") {
    arguments.files = files.map(|s| s.to_owned()).collect();
  }
  if let Some(message) = res.value_of("message") {
    arguments.message = Some(message.to_owned());
  }
  if let Some(service) = res.value_of("service") {
    arguments.service = Some(service.to_owned());
  }
  if let Some(input) = res.value_of("input") {
    arguments.input = Some(input.to_owned());
  }
  if let Some(range) = res.value_of("range") {
    arguments.range = Some(try!(FlexibleRange::parse(range)));
  }
  if let Some(server) = res.value_of("server") {
    if let Some(ref service) = arguments.service {
      if service.to_lowercase() != "hastebin" {
        return Err("--server may only be used if --service is hastebin".into());
      }
    }
    arguments.server = Some(try!(network::parse_url(server).chain_err(|| "invalid --server")));
  }
  if let Some(name) = res.value_of("name") {
    let name = some_or_err!(Path::new(name)
                              .components()
                              .filter_map(|s| {
                                match s {
                                  Component::Normal(x) => Some(x),
                                  _ => None
                                }
                              })
                              .last()
                              .and_then(|s| s.to_str()),
                            "--name had no valid path components".into());
    arguments.name = Some(name.to_owned());
  }
  arguments.raw_urls = res.is_present("raw-urls");
  arguments.urls = res.is_present("urls");
  arguments.all = res.is_present("all");
  arguments.force = res.is_present("force");
  arguments.number_lines = res.is_present("number_lines");
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
