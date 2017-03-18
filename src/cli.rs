use clap::{App, Arg, AppSettings};

pub fn create_app(has_default_bin: bool) -> App<'static, 'static> {
  App::new(crate_name!())
    .about("A tool for pasting from the terminal")
    .author(crate_authors!())
    .version(crate_version!())
    .help_message("print help information and exit")
    .setting(AppSettings::DisableVersion)
    .arg(Arg::with_name("inputs")
      .help("inputs to the program, either files or URLs")
      .takes_value(true)
      .value_name("input")
      .multiple(true))
    .arg(Arg::with_name("debug")
      .long("debug")
      .short("d")
      .help("enable debug output"))
    .arg(Arg::with_name("bin")
      .long("bin")
      .short("b")
      .help("specify the upload bin")
      .required(!has_default_bin)
      .takes_value(true)
      .value_name("bin")
      .possible_values(&["hastebin", "sprunge", "gist"]))
    .arg(Arg::with_name("public")
      .long("public")
      .short("P")
      .help("set the paste to be public")
      .conflicts_with("private"))
    .arg(Arg::with_name("private")
      .long("private")
      .short("p")
      .help("set the paste to be private"))
    .arg(Arg::with_name("authed")
      .long("authed")
      .short("a")
      .help("set the paste to be uploaded while authenticated")
      .conflicts_with("anonymous"))
    .arg(Arg::with_name("anonymous")
      .long("anonymous")
      .short("A")
      .help("set the paste to be uploaded while not authenticated"))
    .arg(Arg::with_name("json")
      .long("json")
      .short("j")
      .help("output JSON information"))
    .arg(Arg::with_name("raw-urls")
      .long("raw-urls")
      .short("r")
      .help("output URLs to the raw content")
      .conflicts_with("html-urls"))
    .arg(Arg::with_name("html-urls")
      .long("html-urls")
      .short("u")
      .help("output URLs to the HTML content"))
    .arg(Arg::with_name("message")
      .long("message")
      .short("m")
      .help("specify a message to upload instead of files or stdin")
      .takes_value(true)
      .value_name("message")
      .conflicts_with("inputs"))
    .arg(Arg::with_name("list-bins")
      .long("list-bins")
      .short("-l")
      .help("list the available bins")
      .conflicts_with_all(&["bin",
        "public",
        "private",
        "anonymous",
        "authed",
        "raw-urls",
        "html-urls",
        "message"]))
      .arg(Arg::with_name("force")
        .long("force")
        .short("f")
        .help("force upload, ignoring safety features"))
      .arg(Arg::with_name("name")
        .long("name")
        .short("N")
        .help("manually set the file name for single-file uploads")
        .takes_value(true)
        .value_name("file_name"))
      .arg(Arg::with_name("version")
        .long("version")
        .short("v")
        .help("print version information and exit")
        .overrides_with("bin"))
}
