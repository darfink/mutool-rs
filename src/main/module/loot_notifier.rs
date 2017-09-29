use std::{time, io};
use std::rc::Rc;
use std::collections::HashMap;
use serde::{Deserialize, Deserializer};
use muonline_packet::{Packet, PacketDecodable};
use mu;
use main::notify::NotificationService;
use ext::{self, model};

/// The module's name.
pub const MODULE: &'static str = "LootNotifier";

/// An implementation of a loot notifier.
struct LootNotifier {
  item_watchlist: HashMap<mu::protocol::model::ItemInfoTiny, (time::Instant, String)>,
  service: Rc<NotificationService>,
  config: Config,
}

impl LootNotifier {
  /// Creates a new loot notifier module.
  unsafe fn new(config: Config, service: Rc<NotificationService>) -> Self {
    LootNotifier {
      config,
      service,
      item_watchlist: HashMap::new(),
    }
  }

  /// Analyzes an item list to notify about any items of interest.
  unsafe fn process_item_list(&mut self, items: &mu::protocol::realm::ItemList) {
    let items = items.iter()
      .map(|entry| model::ItemEntity::from_loot_table((entry.id & 0x7FFF) as usize))
      .filter(|item| self.config.items.iter().any(|f| { f.apply(&item.info) }))
      .collect::<Vec<_>>();

    for item in items {
      // Check if there is a notification delay
      if self.config.delay > 0 {
        // Determine if an item with the same properties is already on the list
        let info: mu::protocol::model::ItemInfoTiny = (&item.info).into();
        let collision = self.item_watchlist.remove(&info);

        match collision {
          Some((_, item_in_list_name)) => {
            // If there is a collision, notify about both items since they cannot be tracked
            self.notify_loot(&item_in_list_name, None);
            self.notify_loot(&item.info.name_with_info(), None);
          },
          None => {
            // Otherwise add the item to the watch list for later notification
            self.item_watchlist.insert(info, (time::Instant::now(), item.info.name_with_info()));
          },
        }
      } else {
        // If there is no delay, notify that an item dropped ASAP
        self.notify_loot(&item.info.name_with_info(), None);
      }
    }
  }

  /// Sends a notification about an item that may have been looted.
  fn notify_loot(&self, loot_info: &str, looter: Option<&str>) {
    let mut message = vec![format!("Item: {}", loot_info)];
    let mut attribute = "Loot";

    if let Some(looter) = looter {
      message.push(format!("User: {}", looter));
      attribute = "Looted";
    }

    let title = format!("Mu Online [{}]", attribute);
    self.service.notify(title, message.join("\n"));
  }

  /// Called whenever a party member or the user picks up an item.
  unsafe fn process_item_pickup(&mut self,
      item: &mu::protocol::model::ItemInfoTiny,
      looter: Option<&model::Entity>) {
    // Check if the item that was obtained is in the watch list
    if let Some((_, item_name_with_info)) = self.item_watchlist.remove(item) {
      // The user's name may not always be available.
      let name = looter.map(|entity| entity.name()).unwrap_or("<unknown>");
      self.notify_loot(&item_name_with_info, Some(name));
    }
  }
}

impl super::Module for LootNotifier {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Processes an incoming server packet.
  unsafe fn process(&mut self, packet: &Packet) {
    // Determine whether this is an item list packet or not
    if let Ok(entries) = mu::protocol::realm::ItemList::from_packet(packet) {
      self.process_item_list(&entries);
    } else if !self.item_watchlist.is_empty() {
      if let Ok(info) = mu::protocol::realm::PartyItemInfo::from_packet(packet) {
        let looter = model::Entity::from_id(info.member_id);
        self.process_item_pickup(&info.item, looter)
      } else if let Ok(result) = mu::protocol::realm::ItemGetResult::from_packet(packet) {
        println!("[LootNotifier] ITEM: {:?}", result);
        println!("[LootNotifier] PACKET: {:?}", packet);
        match result {
          mu::protocol::realm::ItemGetResult::Item { slot: _, item } |
          mu::protocol::realm::ItemGetResult::Stack(item) => {
            println!("[LootNotifier] Item {:?}", item.code());
            let looter = ext::ref_character_entity();
            self.process_item_pickup(&(&item).into(), Some(looter));
          },
          _ => (),
        }
      }
    }
  }

  /// Notifies about item's that has not been looted within the time frame.
  unsafe fn update(&mut self) {
    let delay_duration = time::Duration::from_millis(self.config.delay as u64);
    let is_within_timeframe = move |time: &time::Instant| {
      time.elapsed() < delay_duration
    };

    // Only retain items that are still within their timeframe
    let mut items_removed = Vec::new();
    self.item_watchlist.retain(|_, &mut (time, ref item_name_with_info)| {
      let result = is_within_timeframe(&time);
      if !result { items_removed.push(item_name_with_info.clone()); }
      result
    });

    for item_name_with_info in items_removed {
      self.notify_loot(&item_name_with_info, None);
    }
  }
}

/// Configuration for the notifier.
#[derive(Deserialize, Debug, Default)]
pub struct Config {
  #[serde(default)]
  pub enabled: bool,
  #[serde(default)]
  pub items: Vec<::ItemFilter>,
  #[serde(default)]
  pub delay: u32,
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
    let module = Box::new(LootNotifier::new(self.config, service));

    Ok(module as Box<_>)
  }
}
