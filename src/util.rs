use std::borrow::Cow;
use std::io;
use std::io::prelude::Read;
use std::fs::File;
use std::ffi::{CStr, CString};
use std::path::Path;

pub fn read_file_contents<P: AsRef<Path>>(path: P) -> io::Result<String> {
  let mut file = File::open(path)?;
  let mut contents = String::new();
  file.read_to_string(&mut contents)?;
  Ok(contents)
}

pub fn to_cstr<'a>(text: &'a [u8]) -> Cow<'a, CStr> {
  match CStr::from_bytes_with_nul(text.as_ref()) {
    Ok(data) => Cow::Borrowed(data),
    Err(_) => Cow::Owned(CString::new(text.as_ref()).expect("invalid render string")),
  }
}

pub struct SendPointer<T: ?Sized>(pub *mut T);

unsafe impl<T> Send for SendPointer<T> { }