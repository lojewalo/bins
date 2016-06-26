extern crate magic_sys;
extern crate std;

use bins::error::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub struct MagicWrapper {
  magic: *const magic_sys::Magic,
  loaded: bool
}

cfg_if! {
  if #[cfg(target_pointer_width = "64")] {
    type SizeT = u64;
  } else if #[cfg(target_pointer_width = "32")] {
    type SizeT = u32;
  } else {
    // well, that's interesting. default down to 32
    type SizeT = u32;
  }
}

impl Drop for MagicWrapper {
  fn drop(&mut self) {
    unsafe { magic_sys::magic_close(self.magic) }
  }
}

impl MagicWrapper {
  pub fn new(flags: i32, load_defaults: bool) -> Result<Self> {
    let magic = unsafe { magic_sys::magic_open(flags) };
    if magic == std::ptr::null() {
      return Err("libmagic could not create a new magic cookie".into());
    }
    let mut wrapper = MagicWrapper {
      magic: magic,
      loaded: false
    };
    if load_defaults {
      try!(wrapper.load(None));
    }
    Ok(wrapper)
  }

  pub fn load(&mut self, paths: Option<String>) -> Result<()> {
    let ptr = match paths {
      Some(p) => try!(CString::new(p).map_err(|e| e.to_string())).as_ptr(),
      None => std::ptr::null(),
    };
    let load_status = unsafe { magic_sys::magic_load(self.magic, ptr) };
    if load_status != 0 {
      return Err("libmagic could not load default magic database".into());
    }
    self.loaded = true;
    Ok(())
  }

  fn check_loaded(&self) -> Result<()> {
    if !self.loaded {
      return Err("magic database was not loaded".into());
    }
    Ok(())
  }

  fn check_magic_return_value(&self, ptr: *const c_char) -> Result<String> {
    if ptr == std::ptr::null() {
      let error = unsafe { magic_sys::magic_error(self.magic) };
      if error == std::ptr::null() {
        Err("libmagic had an error but didn't think it did".into())
      } else {
        let error = unsafe { CStr::from_ptr(error) };
        Err(format!("libmagic error: {}", error.to_string_lossy().into_owned()).into())
      }
    } else {
      Ok(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }
  }

  pub fn magic_buffer(&self, buf: &[u8]) -> Result<String> {
    try!(self.check_loaded());
    let info: *const c_char = unsafe { magic_sys::magic_buffer(self.magic, buf.as_ptr(), buf.len() as SizeT) };
    self.check_magic_return_value(info)
  }

  // pub fn magic_file<S>(&self, path: S) -> Result<String> where S: Into<String> {
  //   try!(self.check_loaded());
  //   let path = try!(CString::new(path.into()).map_err(|e| e.to_string()));
  //   let info: *const c_char = unsafe { magic_sys::magic_file(self.magic, path.as_ptr()) };
  //   self.check_magic_return_value(info)
  // }
}
