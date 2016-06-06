use argparse::{ArgumentParser, Store, List};

pub struct Arguments {
  pub files: Vec<String>,
  pub message: String,
  pub service: String
}

pub fn get_arguments() -> Arguments {
  let mut arguments = Arguments {
    files: Vec::new(),
    message: String::from(""),
    service: String::from("")
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
    ap.parse_args_or_exit();
  }
  arguments
}
