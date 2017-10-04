use std::io;
use muonline_packet::{Packet, PacketDecodable, PacketEncodable};
use serde::{Deserialize, Deserializer};
use mu;
use ext::{self, model};

/// The module's name.
pub const MODULE: &'static str = "AutoHealthPotion";

struct AutoHealthPotion {
  config: Config,
}

impl AutoHealthPotion {
  /// Creates a new health potion module.
  unsafe fn new(config: Config) -> Self {
    AutoHealthPotion { config }
  }

  /// Uses a health potion from the user's inventory.
  unsafe fn use_health_potion(&self) {
    let inventory = &model::Gobj::get().inventory;

    // Find any usable potion, and prioritize higher level ones
    let mut potions = (0..4).rev().filter_map(|index| {
      let code = mu::ItemCode::new(mu::ItemGroup::Potion, index as u16);
      inventory.get_item_slot_from_code(code, None).map(|slot| (code, slot))
    });

    if let Some((code, slot)) = potions.next() {
      // Notify the user that an automatic potion has been consumed
      let message = format!("Using {}", ext::func::get_item_name(code, 0));
      ext::func::show_notice(message);

      // If a potion could be found, use it now
      mu::protocol::client::UseItem::new(slot + 12)
        .to_packet()
        .and_then(|packet| ext::func::send_packet(&packet, true))
        .expect("sending potion use request");
      *ext::ref_using_potion_type() = 10;
    }
  }
}

impl super::Module for AutoHealthPotion {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Analyzes a packet to detect any item loot of interest.
  unsafe fn process(&mut self, packet: &Packet) {
    // Return if the user is currently using a potion
    if *ext::ref_using_potion_type() > 0 {
      return;
    }

    if let Ok(attack) = mu::protocol::realm::PlayerDamageExt::from_packet(packet) {
      let character = model::Character::get();
      let ratio = (character.health as f32) / (character.max_health as f32);

      let victim_is_player = attack.victim_id == ext::ref_character_entity().id;
      let health_is_below_threshold = ratio <= self.config.threshold;

      if victim_is_player && health_is_below_threshold {
        self.use_health_potion();
      }
    }
  }
}

/// Configuration for the auto health potion.
#[derive(Serialize, Deserialize, Debug)]
struct Config {
  #[serde(default)]
  pub enabled: bool,
  pub threshold: f32,
}

pub struct Builder {
  config: Config,
}

impl Builder {
  /// Constructs a new builder.
  pub fn new<'de, D>(config: D) -> io::Result<Self> where D: Deserializer<'de> {
    Ok(Builder {
      config: Config::deserialize(config)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?
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
    let module = Box::new(AutoHealthPotion::new(self.config));
    Ok(module as Box<_>)
  }
}