use std::process;
use clap::{App, Arg};
use config::types::Config;

pub struct Arguments {
  pub files: Vec<String>,
  pub message: Option<String>,
  pub service: Option<String>,
  pub private: bool,
  pub auth: bool,
  pub input: Option<String>
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

pub fn get_arguments(config: &Config) -> Arguments {
  let mut arguments = Arguments {
    files: Vec::new(),
    message: None,
    service: config.lookup_str("defaults.service").map(|s| s.to_owned()),
    private: config.lookup_boolean_or("defaults.private", true),
    auth: config.lookup_boolean_or("default.auth", true),
    input: None
  };
  let name = get_name();
  let version = get_version();
  let res = App::new(name.as_ref())
    .version(version.as_ref())
    .about("A command-line pastebin client")
    .arg(Arg::with_name("files")
         .help("files to paste")
         .takes_value(true)
         .multiple(true))
    .arg(Arg::with_name("message")
         .short("m")
         .long("message")
         .help("message to paste")
         .use_delimiter(false)
         .takes_value(true))
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
         .conflicts_with_all(&["auth", "anon", "public", "private", "message", "service"]))
    .get_matches();
  if res.is_present("list-services") {
    println!("gist\nhastebin\npastebin\npastie\nsprunge");
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
  arguments
}
