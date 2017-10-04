use std::{io, mem};
use std::cmp::Ordering;
use tap::{TapBooleanOps, TapOptionOps};
use serde::{Deserialize, Deserializer};
use detour::{Detour, StaticDetour};
use muonline_packet::{Packet, PacketDecodable, PacketEncodable};
use ext::{self, model};

/// The module's name.
pub const MODULE: &'static str = "LootFilter";

/// An implementation of a loot filter.
struct LootFilter {
  loot_items: Vec<&'static model::ItemEntity>,
  config: Config,
}

impl LootFilter {
  /// Creates a new loot filter module.
  unsafe fn new(config: Config) -> Self {
    LootFilter { loot_items: Vec::new(), config }
  }

  /// Called whenever the user tries to loot an item using *space*.
  unsafe fn on_item_pickup(&mut self) {
    // Remove all items that have been picked up or expired
    self.loot_items.retain(|item| item.is_active);

    // Try to find a lootable item within range
    let inventory = &model::Gobj::get().inventory;
    let entry = self.loot_items.iter()
      .filter(|item| item.is_on_ground)
      .filter(|item| item.is_within_loot_range())
      .filter(|item| inventory
        .has_space_for_item(&item.info)
        .tap_false(|_| ext::func::show_notice(
          format!("Your bag cannot store {}", &item.info.name()))))
      .next();

    if let Some(item) = entry {
      self.pickup_item(item);
    }
  }

  /// Sends an item pickup request to the server.
  pub unsafe fn pickup_item(&self, item: &model::ItemEntity) {
    // Abort if there is a pending network request
    if *ext::ref_item_loot_request() != u32::max_value() {
      return;
    }

    let index = try_opt!(ext::ref_loot_table().iter()
      .position(|other| ::std::ptr::eq(other, item))
      .tap_none(|| eprintln!("[LootFilter:Error] Failed to retrieve item index")));

    *ext::ref_item_loot_request() = index as u32;
    *ext::ref_item_loot_index() = index as u32;

    ::mu::protocol::client::ItemGetRequest::new(index as u16).to_packet()
      .and_then(|packet| ext::func::send_packet(&packet, true))
      .expect("sending item request packet");
  }

  /// Fiters out item's that are not of interest.
  fn filter_item(&self, item: &model::Item) -> bool {
    // TODO: This "is_zen" should not be necessary
    item.is_zen() || self.config.items.iter().any(|f| f.apply(item))
  }

  #[cfg(windows)]
  unsafe fn ignore_filter(&self) -> bool {
    (::user32::GetAsyncKeyState(::winapi::winuser::VK_SHIFT) & -0x8000) != 0
  }

  #[cfg(unix)]
  unsafe fn ignore_filter(&self) -> bool { unimplemented!(); }
}

impl super::Module for LootFilter {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Processes an incoming server packet.
  unsafe fn process(&mut self, packet: &Packet) {
    // Determine whether this is an item list packet or not
    if let Ok(entries) = ::mu::protocol::realm::ItemList::from_packet(packet) {
      // Retrieve the item's from memory and apply a filter
      let items = entries.iter()
        .map(|entry| model::ItemEntity::from_loot_table((entry.id & 0x7FFF) as usize))
        .filter(|item| self.filter_item(&item.info))
        .collect::<Vec<_>>();

      self.loot_items.extend(items);
      self.loot_items.sort_unstable_by(|a, b| {
        // Zen should always be looted last
        match (a.info.is_zen(), b.info.is_zen()) {
          (false, true) => Ordering::Less,
          (true, false) => Ordering::Greater,
          _ => Ordering::Equal,
        }
      });
    }
  }
}

/// Configuration for the filter.
#[derive(Deserialize, Debug, Default)]
pub struct Config {
  #[serde(default)]
  enabled: bool,
  #[serde(default)]
  items: Vec<::ItemFilter>,
}

static_detours! {
  // The detour for an entity action, triggered whenever space is pressed.
  struct DetourEntityAction: extern "C" fn(*const model::Entity, *const model::ItemEntity, bool);
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

  /// Initializes the detour used for loot filtering.
  unsafe fn init_hook(
      filter: *mut LootFilter,
      target: ext::EntityAction)
      -> io::Result<StaticDetour<ext::EntityAction>> {
    let filter = ::util::SendPointer(filter);
    let detour = move |character: *const model::Entity, item: *const model::ItemEntity, is_loot| {
      if !*ext::ref_item_loot_ranged() || (*filter.0).ignore_filter() {
        DetourEntityAction.get()
          .expect("retrieving detour controller")
          .call(character, item, is_loot);
      } else {
        (*filter.0).on_item_pickup();
      }
    };

    DetourEntityAction.initialize(target, detour)
      .and_then(|mut hook| hook.enable().map(|_| hook))
      .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
  }
}

impl super::ModuleBuilder for Builder {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Returns whether this module is enabled or not.
  fn enabled(&self) -> bool { self.config.enabled }

  /// Builds the module associated with this builder.
  unsafe fn build(self: Box<Self>) -> io::Result<Box<super::Module>> {
    // TODO: How to handle detours in case the module is disabled?
    let mut module = Box::new(LootFilter::new(self.config));
    mem::forget(Self::init_hook(&mut *module, ext::ref_entity_action())?);
    Ok(module as Box<_>)
  }
}

// TODO: Implement instant pickup!!! NOTE: CHECK FOR INVENTORY SPACE
/*if self.config.instant_pickup {
  // Instant zen may create suspiciousness
  let mut lootable_items = items.iter()
    .filter(|item| item.is_active)
    .filter(|item| item.is_within_loot_range())
    .filter(|item| !item.is_zen());

  // Only one item can be looted at a time
  if let Some(item) = lootable_items.next() {
    Self::pickup_item(item);
  }
}*/
