use std::{thread, io};
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Deserializer};
use knock::HTTP;
use tap::TapResultOps;
use super::NotificationService;

/// Configuration for `Pushbullet`.
#[derive(Serialize, Deserialize, Debug)]
struct Config {
  pub access_token: String,
}

/// An implementation of Pushbullet notification.
pub struct Pushbullet {
  config: Config,
}

impl Pushbullet {
  /// Creates a new client notifier
  pub fn new<'de, D>(config: D) -> io::Result<Self> where D: Deserializer<'de> {
    Ok(Pushbullet {
      config: Config::deserialize(config)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?
    })
  }

  /// Sends a notification using a token.
  fn notify_impl<S: Into<String>>(token: S, title: &str, message: &str) -> io::Result<()> {
    let mut http = HTTP::new("https://api.pushbullet.com/v2/pushes")
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
    let mut headers = HashMap::new();
    headers.insert("Access-Token".into(), token.into());
    headers.insert("Content-Type".into(), "application/json".into());

    let json = json!({
        "title": title,
        "body": message,
        "type": "note"
    });

    http.post()
        .body_as_str(&json.to_string())
        .header(headers)
        .send()
        .map(|_| ())
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
  }
}

impl NotificationService for Pushbullet {
  /// Sends a notification inside a new thread.
  fn notify(&self, title: String, message: String) -> thread::JoinHandle<io::Result<()>> {
    let token = self.config.access_token.clone();

    thread::spawn(move || {
      Self::notify_impl(token, &title, &message).tap_err(|error| {
        eprintln!("[Notify:Error] Failed to send notification: {}", error);
      })
   })
  }

  fn notify_jpg(&self, _title: String, _message: String, _image: PathBuf) -> thread::JoinHandle<io::Result<()>> {
    unimplemented!();
    /*let token = self.config.access_token.clone();

    thread::spawn(move || {
      let file_name: String = image.file_name()
        .expect("retrieving jpg file name")
        .to_string_lossy()
        .into();

      // Ensure that the file is accessable
      let form = reqwest::multipart::Form::new().file("file", image)?;

      let request = move |client: reqwest::Client| {
        // Request to upload a file
        let info: UploadRequest = client
          .post("https://api.pushbullet.com/v2/upload-request")
          .header(AccessToken(token.clone()))
          .json(&map!["file_name" => file_name, "file_type" => "image/jpeg".into()])
          .send()?
          .json()?;

        println!("2");
        println!("{:?}", info);
        // Upload the file contents
        client
          .post(&info.upload_url)
          .multipart(form)
          .send()?;

        // Send the file notification
        let json = map![
          "title" => title,
          "body" => message,
          "type" => "file".to_owned(),
          "file_name" => info.file_name,
          "file_type" => info.file_type,
          "file_url" => info.file_url
        ];

        client
          .post("https://api.pushbullet.com/v2/pushes")
          .header(AccessToken(token))
          .json(&json)
          .send()
      };

      request(reqwest::Client::new())
        .tap_err(|error| eprintln!("Failed to send image notification: {}", error))
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
        .map(|_| ())
    })*/
  }
}

#[derive(Deserialize, Debug)]
pub struct UploadRequest {
  pub file_name: String,
  pub file_type: String,
  pub file_url: String,
  pub upload_url: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  /*#[test]
  fn notify() {
    let config = Config { access_token: "o.Tts0zcmOgc9mT7UA4zIE5DEg1BbhnJRP".into() };
    let client = Pushbullet::new(config);
    client.notify("Mu Online [Test]".into(), "X Sphinx Pants+L+dd+dsr".into())
      .join()
      .expect("joining thread")
      .expect("sending notice");
  }

  #[test]
  fn notify_jpg() {
    let config = Config { access_token: "o.Tts0zcmOgc9mT7UA4zIE5DEg1BbhnJRP".into() };
    let client = Pushbullet::new(config);
    client.notify_jpg(
      "Mu Online [Test]".into(),
      "Attacker: Dwarf".into(),
      "./res/screenshot.jpg".into())
    .join()
    .expect("joining thread")
    .expect("sending jpg");
  }*/
}
