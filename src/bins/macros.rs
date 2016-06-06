macro_rules! some_or_err {
  ($expr: expr, $err: expr) => {
    match $expr { Some(x) => x, None => return Err($err) }
  }
}
