use std::io;
use muonline_packet::{Packet, PacketDecodable, PacketEncodable};
use serde::{Deserialize, Deserializer};
use mu;
use num_traits::FromPrimitive;
use ext::{self, model};

/// The module's name.
pub const MODULE: &'static str = "AutoRepair";

/// An implementation of an auto repairer.
struct AutoRepair(());

impl AutoRepair  {
  /// Creates a new auto repair module.
  unsafe fn new(_: Config) -> Self {
    AutoRepair(())
  }
}

impl super::Module for AutoRepair {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Analyzes a packet to detect any item loot of interest.
  unsafe fn process(&mut self, packet: &Packet) {
    // Check whether any item has been damaged or not
    if let Ok(info) = mu::protocol::realm::ItemDurability::from_packet(packet) {
      // Only equipped items are of interest
      if let Some(slot) = mu::EquipmentSlot::from_u8(info.inventory_index) {
        // Helpers, such as imp and uniria cannot be repaired
        if slot == mu::EquipmentSlot::Helper { return; }

        // This is a tool, not a cheat, so abide the level requirement
        if model::Character::get().level < 50 { return; }

        if let Some(item) = model::Item::from_equipment(slot) {
          // Arrows and bolts are not a repairable item
          if item.code().group() == mu::ItemGroup::Bow
            && matches!(item.code().id(), 7 | 15) { return; }

          // The game does not do any bookmarking
          item.update_durability_status();

          // If the item durability is perfect there's no point in repairing it
          if item.status == model::DurabilityStatus::Perfect { return; }

          // Notify the user that an automatic repair has been triggered
          ext::func::show_notice(format!("Repairing {}", slot));
          println!("[AutoRepair] Repairing {:?} | Code: {}", slot, item.code);

          // Send the repair request to the server
          mu::protocol::client::ItemRepairRequest {
            inventory_index: info.inventory_index,
            standalone_repair: true,
          }.to_packet().and_then(|packet| {
            ext::func::send_packet(&packet, true)
          }).expect("sending item repair request");
        }
      }
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
  #[serde(default)]
  pub enabled: bool,
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
    let module = Box::new(AutoRepair::new(self.config));
    Ok(module as Box<_>)
  }
}