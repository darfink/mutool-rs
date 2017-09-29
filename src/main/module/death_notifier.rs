use std::io;
use std::rc::Rc;
use serde::{Deserialize, Deserializer};
use muonline_packet::{Packet, PacketDecodable};
use main::notify::NotificationService;
use mu;
use ext::{self, model};

/// The module's name.
pub const MODULE: &'static str = "DeathNotifier";

/// An implementation of a death notifier.
struct DeathNotifier {
  config: Config,
  service: Rc<NotificationService>,
}

impl DeathNotifier {
  /// Creates a new death notifier instance.
  fn new(config: Config, service: Rc<NotificationService>) -> Self {
    DeathNotifier { config, service }
  }
}

impl super::Module for DeathNotifier {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Analyzes a packet to detect if the player dies.
  unsafe fn process(&mut self, packet: &Packet) {
    if let Ok(event) = mu::protocol::realm::PlayerDeath::from_packet(packet) {
      let victim = model::Entity::from_id(event.victim_id)
        .expect("retrieving victim entity");

      if victim.id == ext::ref_character_entity().id {
        let attacker = model::Entity::from_id(event.attacker_id)
          .expect("retrieving attacker entity");

        let message = format!("Attacker: {}", attacker.name());
        let title = "Mu Online [Killed]".to_owned();

        if self.config.screenshot {
          ext::ref_print_screen()();
        }

        self.service.notify(title, message);
      }
    }
  }
}

/// Configuration for `DeathNotifier`.
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  #[serde(default)]
  pub enabled: bool,
  #[serde(default)]
  pub screenshot: bool,
}

pub struct Builder {
  service: Rc<NotificationService>,
  config: Config,
}

impl Builder {
  /// Constructs a new builder.
  pub fn new<'de, D>(config: D, service: Rc<NotificationService>) -> io::Result<Self>
      where D: Deserializer<'de> {
    Ok(Builder {
      config: Config::deserialize(config)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?,
      service,
    })
  }
}

impl super::ModuleBuilder for Builder {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Returns whether this module is enabled or not.
  fn enabled(&self) -> bool { self.config.enabled }

  /// Builds the module associated with this builder.
  unsafe fn build(self: Box<Self>) -> io::Result<Box<super::Module>> {
    let service = self.service.clone();
    let module = Box::new(DeathNotifier::new(self.config, service));
    
    Ok(module as Box<_>)
  }
}
