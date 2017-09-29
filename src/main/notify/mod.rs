use std::{thread, io};
use std::path::PathBuf;

pub mod pushbullet;
pub use self::pushbullet::Pushbullet;

pub trait NotificationService {
  /// Notifies a client with a title and message.
  fn notify(&self, title: String, message: String) -> thread::JoinHandle<io::Result<()>>;
  fn notify_jpg(&self, title: String, message: String, image: PathBuf) -> thread::JoinHandle<io::Result<()>>;
}
