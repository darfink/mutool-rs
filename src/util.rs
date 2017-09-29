use std::io;
use std::io::prelude::Read;
use std::fs::File;
use std::path::Path;

pub fn read_file_contents<P: AsRef<Path>>(path: P) -> io::Result<String> {
  let mut file = File::open(path)?;
  let mut contents = String::new();
  file.read_to_string(&mut contents)?;
  Ok(contents)
}

pub struct SendPointer<T: ?Sized>(pub *mut T);

unsafe impl<T> Send for SendPointer<T> { }