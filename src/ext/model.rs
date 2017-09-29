#![allow(dead_code)]
use std::mem;
use std::ffi::CStr;
use std::fmt::Write;
use std::os::raw::c_char;
use mu;

#[repr(packed)]
pub struct Gobj {
  pad0: [u8; 40],
  pub inventory: &'static mut Inventory,
}

impl Gobj {
  pub unsafe fn get() -> &'static mut Self {
    super::ref_get_gobj()().as_mut().expect("retreiving global object")
  }
}

#[repr(packed)]
pub struct Renderer {
  pad0: [u8; 4],
  pub device: *mut (),
}

impl Renderer {
  pub unsafe fn get() -> &'static mut Self {
    super::ref_get_renderer()().as_mut().expect("retreiving renderer")
  }

  pub unsafe fn draw_rectangle(&self, x: f32, y: f32, width: f32, height: f32, color: mu::Color) {
    self.set_gl_color(color);
    super::ref_draw_rectangle()(x, y, width, height);
  }

  pub unsafe fn draw_text<S: AsRef<[u8]>>(&self, text: S, x: i32, y: i32, color: mu::Color, background: mu::Color) {
    use std::ffi::{CStr, CString};
    use std::borrow::Cow;

    type DrawText = extern "thiscall" fn(*const Renderer, i32, i32, *const c_char, i32, i32, i32, i32);
    let method: DrawText = mem::transmute(0x41D9F1u32);

    let text = match CStr::from_bytes_with_nul(text.as_ref()) {
      Ok(data) => Cow::Borrowed(data),
      Err(_) => Cow::Owned(CString::new(text.as_ref()).expect("invalid render string")),
    };

    self.set_text_color(color);
    self.set_text_background(background);
    method(self, x, y, text.as_ptr(), 0, 0, 1, 0);
  }

  unsafe fn set_text_background(&self, color: mu::Color) {
    let method: extern "thiscall" fn(*const Renderer, u8, u8, u8, u8) = mem::transmute(0x41D964u32);
    method(self, color.red, color.green, color.blue, color.alpha);
  }

  unsafe fn set_text_color(&self, color: mu::Color) {
    let method: extern "thiscall" fn(*const Renderer, u8, u8, u8, u8) = mem::transmute(0x41D902u32);
    method(self, color.red, color.green, color.blue, color.alpha);
  }

  unsafe fn set_gl_color(&self, color: mu::Color) {
    let function: *const extern "system" fn(f32, f32, f32, f32) = mem::transmute(0x812390u32);
    (*function)(
      (color.red as f32) / 255.0,
      (color.green as f32) / 255.0,
      (color.blue as f32) / 255.0,
      (color.alpha as f32) / 255.0);
  }
}

pub struct Inventory(u32);

impl Inventory {
  pub unsafe fn get_item_slot_from_code(&self, code: mu::ItemCode, level: Option<u8>) -> Option<u8> {
    let method: extern "thiscall" fn(*const Inventory, u16, i32) -> i32 = mem::transmute(0x734490u32);
    match method(self, code.as_raw(), level.map(|l| l as i32).unwrap_or(-1)) {
      -1 => None,
      slot => Some(slot as u8),
    }
  }

  pub unsafe fn find_slot_for_item(&self, item: &Item) -> Option<u8> {
    let method: extern "thiscall" fn(*const Inventory, *const Item) -> i32 = mem::transmute(0x7344EDu32);
    match method(self, item) {
      -1 => None,
      slot => Some(slot as u8)
    }
  }

  pub unsafe fn has_space_for_item(&self, item: &Item) -> bool {
    item.is_zen() || self.find_slot_for_item(item).is_some()
  }
}

#[repr(packed)]
pub struct User {
  pad0: [u8; 4320],
  pub equipment: [Item; 12],
}

#[repr(packed)]
pub struct Vector {
  pub x: f32,
  pub y: f32,
  pub z: f32,
}

impl Vector {
  /// Calculates the distance between two vectors
  pub fn distance(&self, other: &Vector) -> f32 {
    ((self.x - other.x).powi(2) +
     (self.y - other.y).powi(2) +
     (self.z - other.z).powi(2)).sqrt()
  }
}

#[repr(packed)]
pub struct Character {
  pad0: [u8; 14],
  pub level: u16,
  pub experience: u32,
  pub experience_next: u32,
  pub strength: u16,
  pub agility: u16,
  pub vitality: u16,
  pub energy: u16,
  pub command: u16,
  pub health: u16,
  pub mana: u16,
  pub max_health: u16,
  pub max_mana: u16,
  pub shield: u16,
}

impl Character {
  pub unsafe fn get() -> &'static mut Self {
    super::ref_character()
  }
}

#[repr(packed)]
pub struct Entity {
  pad0: [u8; 56],
  pub name: [c_char; 10],
  pad1: [u8; 15],
  pub action: u8,
  pad2: [u8; 4],
  pub id: u16,
  pad3: [u8; 528],
  pub is_active: bool,
  pad4: [u8; 247],
  pub position: Vector,
  pad5: [u8; 248],
}

impl Entity {
  pub unsafe fn from_id(id: u16) -> Option<&'static Self> {
    super::ref_entity_table().iter().find(|entity| entity.is_active && entity.id == id)
  }

  pub unsafe fn character() -> &'static mut Self {
    super::ref_character_entity()
  }

  pub fn name(&self) -> &str {
    unsafe {
      CStr::from_ptr(self.name.as_ptr())
        .to_str()
        .expect("retrieving entity name")
    }
  }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd)]
pub enum DurabilityStatus {
  Perfect   = 0,
  Scratched = 1,
  Dented    = 2,
  Damaged   = 3,
  Shattered = 4,
  Destroyed = 5,
}

#[repr(packed)]
pub struct Item {
  pub code: u16,
  pub modifier: u32,
  pad1: [u8; 16],
  pub durability: u8,
  pub excellent: u8,
  pub ancient: u8,
  pad2: [u8; 63],
  pub status: DurabilityStatus,
  pad3: [u8; 4],
}

impl Item {
  pub unsafe fn from_equipment(slot: mu::EquipmentSlot) -> Option<&'static mut Item> {
    let item = &mut (*super::ref_user()).equipment[slot as usize];
    match (item.code & 0x8000) == 0x8000 {
      false => Some(item),
      true => None,
    }
  }

  pub unsafe fn update_durability_status(&mut self) {
    super::ref_update_item_durability_status()(self);
  }

  pub fn code(&self) -> mu::ItemCode { mu::ItemCode::from_code(self.code) }
  pub fn level(&self) -> u8 { ((self.modifier() >> 3) & 0xF) as u8 }
  pub fn option(&self) -> u8 { (self.modifier() & 0x3) as u8 + ((self.excellent >> 4) & 0x4) }
  pub fn excellent(&self) -> u8 { self.excellent & 0x3F }
  pub fn has_skill(&self) -> bool { ((self.modifier() >> 7) & 0x1) > 0 }
  pub fn has_luck(&self) -> bool { ((self.modifier() >> 2) & 0x1) > 0 }
  pub fn is_zen(&self) -> bool { self.code == 7183 }
  pub fn is_excellent(&self) -> bool { self.excellent() > 0 }
  pub fn is_ancient(&self) -> bool { matches!(self.ancient % 4, 1 | 2) }
  pub fn is_valuable(&self) -> bool {
    self.is_excellent() ||
    self.is_ancient() ||
    (self.code().group() <= mu::ItemGroup::Boots && self.level() > 4)
  }

  pub unsafe fn name(&self) -> String {
    // In case it is an equippable item, exclude the level in the base name
    let level = if self.code().group() <= mu::ItemGroup::Boots { 0 } else { self.level() };
    super::func::get_item_name(self.code(), level)
  }

  pub unsafe fn name_with_info(&self) -> String {
    if self.is_zen() {
      return format!("Zen {}", self.modifier);
    }

    let mut output = String::with_capacity(30);

    if self.is_excellent() {
      output.push_str("X ");
    }

    output.push_str(&self.name());

    let group = self.code().group();
    if group <= mu::ItemGroup::Boots && (self.option() > 0 || self.level() > 0) {
      write!(&mut output, "+{}", self.level()).expect("appending item level");
    }

    if self.option() > 0 {
      if group == mu::ItemGroup::Helper {
        write!(&mut output, "+{}%", self.option()).unwrap();
      } else if group == mu::ItemGroup::Shield {
        write!(&mut output, "+{}", self.option() * 5).unwrap();
      } else if group <= mu::ItemGroup::Boots {
        write!(&mut output, "+{}", self.option() * 4).unwrap();
      }
    }

    if self.has_skill() {
      output.push_str("+S");
    }

    if self.has_luck() {
      output.push_str("+L");
    }

    if self.is_excellent() && group <= mu::ItemGroup::Boots {
      const OFFENSIVE: &[&'static str] = &["mana8", "hp8", "speed", "dmg", "dmg20", "xdmg"];
      const DEFENSIVE: &[&'static str] = &["zen", "dsr", "ref", "dd", "mana", "hp"];

      let excellent = self.excellent();
      let ex_options = if group <= mu::ItemGroup::Staff {
        OFFENSIVE
      } else {
        DEFENSIVE
      };

      for (index, ex_option) in ex_options.iter().enumerate() {
        if (excellent & (1 << index)) != 0 {
          write!(&mut output, "+{}", ex_option).unwrap();
        }
      }
    }

    output
  }

  /// Returns the modifier or zero if the item is zen.
  fn modifier(&self) -> u32 {
    if self.is_zen() { 0 } else { self.modifier }
  }
}

impl<'a> From<&'a Item> for mu::protocol::model::ItemInfoTiny {
  fn from(item: &Item) -> Self {
    let mut info = mu::protocol::model::ItemInfoTiny::new();
    info.set_has_luck(item.has_luck());
    info.set_has_skill(item.has_skill());
    info.set_has_option(item.option() > 0);
    info.set_is_excellent(item.is_excellent());
    info.set_is_ancient(item.is_ancient());
    info.set_level(item.level());
    info.set_code(item.code());
    info
  }
}

#[repr(packed)]
pub struct ItemEntity {
  pub info: Item,
  pad1: [u8; 5],
  pub is_active: bool,
  pad2: [u8; 7],
  pub is_on_ground: bool,
  pad3: [u8; 35],
  pub object_type: u32,
  pub lifetime: u32,
  pad4: [u8; 196],
  pub position: Vector,
  pad5: [u8; 250],
}

impl ItemEntity {
  pub unsafe fn from_loot_table(id: usize) -> &'static Self {
    let loot_table = super::ref_loot_table();
    let id = if id >= loot_table.len() { 0 } else { id };
    &loot_table[id]
  }

  pub unsafe fn is_within_loot_range(&self) -> bool {
    const MAX_RANGE: f32 = 300.0;
    let character = super::ref_character_entity();
    self.position.distance(&character.position) < MAX_RANGE
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::mem;

  macro_rules! offset_of {
    ($ty:ty => $field:ident) => {
      unsafe { &(*(0 as *const $ty)).$field as *const _ as usize }
    }
  }

  #[test]
  fn struct_character() {
    assert_eq!(offset_of!(Character => experience), 0x10);
    assert_eq!(offset_of!(Character => experience_next), 0x14);
    assert_eq!(offset_of!(Character => health), 0x22);
  }

  #[test]
  fn struct_entity() {
    assert_eq!(offset_of!(Entity => name), 0x38);
    assert_eq!(offset_of!(Entity => action), 0x51);
    assert_eq!(offset_of!(Entity => id), 0x56);
    assert_eq!(offset_of!(Entity => is_active), 0x268);
    assert_eq!(offset_of!(Entity => position), 0x360);
    assert_eq!(mem::size_of::<Entity>(), 0x464);
  }

  #[test]
  fn struct_item() {
    assert_eq!(offset_of!(Item => code), 0x00);
    assert_eq!(offset_of!(Item => durability), 0x16);
    assert_eq!(offset_of!(Item => status), 0x58);
    assert_eq!(mem::size_of::<Item>(), 0x5D);
  }

  #[test]
  fn struct_item_entity() {
    assert_eq!(offset_of!(ItemEntity => is_active), 0x62);
    assert_eq!(offset_of!(ItemEntity => is_on_ground), 0x6A);
    assert_eq!(offset_of!(ItemEntity => object_type), 0x8E);
    assert_eq!(offset_of!(ItemEntity => position), 0x15A);
    assert_eq!(mem::size_of::<ItemEntity>(), 0x260);
  }
}
