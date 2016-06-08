use argparse::{ArgumentParser, Store, List, StoreTrue, StoreFalse, Print};
use config::types::Config;

pub struct Arguments {
  pub files: Vec<String>,
  pub message: String,
  pub service: String,
  pub private: bool,
  pub auth: bool
}

pub fn get_arguments(config: &Config) -> Arguments {
  let mut arguments = Arguments {
    files: Vec::new(),
    message: String::from(""),
    service: String::from(""),
    private: config.lookup_boolean_or("defaults.private", true),
    auth: config.lookup_boolean_or("default.auth", true)
  };
  {
    let mut ap = ArgumentParser::new();
    ap.set_description("paste a file, string, or pipe to a pastebin");
    ap.refer(&mut arguments.files)
      .add_argument("files", List, "files to paste")
      .required();
    ap.refer(&mut arguments.service)
      .add_option(&["-s", "--service"], Store, "pastebin service to use")
      .required();
    ap.refer(&mut arguments.message)
      .add_option(&["-m", "--message"], Store, "message to paste");
    ap.refer(&mut arguments.private)
      .add_option(&["-p", "--private"], StoreTrue, "if the paste should be private")
      .add_option(&["-P", "--public"], StoreFalse, "if the paste should be public");
    ap.refer(&mut arguments.auth)
      .add_option(&["-a", "--auth"], StoreTrue, "if authentication (like api keys and tokens) should be used")
      .add_option(&["-A", "--anon"], StoreFalse, "if pastes should be posted without authentication");
    ap.add_option(
      &["-l", "--list-services"],
      Print(String::from("gist, hastebin, pastebin, pastie")),
      "lists pastebin services available"
    );
    ap.parse_args_or_exit();
  }
  arguments
}
