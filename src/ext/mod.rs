use std::os::raw::c_char;
use std::{mem, slice};

pub mod model;
pub mod func;

pub type GetGobj = extern "C" fn() -> *mut model::Gobj;
pub type GetRenderer = extern "C" fn() -> *mut model::Renderer;
pub type DrawRectangle = extern "C" fn(f32, f32, f32, f32);
pub type UpdateItemDurabilityStatus = extern "system" fn(*mut model::Item);
pub type ReceiveNotice = extern "C" fn(*const u8);
pub type StateWorldRender = extern "C" fn();
pub type LoadWorld = extern "C" fn(u32);
pub type SendPacket = extern "C" fn(data: *const u8, size: usize, encrypt: u32, force_c4: u32) -> bool;
pub type ProtocolCore = extern "C" fn(u32, *mut u8, u32, bool) -> bool;
pub type GetItemName = extern "C" fn(u16, u8, *mut c_char) -> bool;
pub type EntityAction = extern "C" fn(*const model::Entity, *const model::ItemEntity, bool);
pub type PrintScreen = extern "C" fn();

pub unsafe fn ref_get_gobj() -> GetGobj { mem::transmute(0x750101u32) }
pub unsafe fn ref_get_renderer() -> GetRenderer { mem::transmute(0x41D732u32) }
pub unsafe fn ref_draw_rectangle() -> DrawRectangle { mem::transmute(0x5E093Cu32) }
pub unsafe fn ref_update_item_durability_status() -> UpdateItemDurabilityStatus { mem::transmute(0x712A6Du32) }
pub unsafe fn ref_receive_notice() -> ReceiveNotice { mem::transmute(0x603490u32) }
pub unsafe fn ref_state_world_render() -> StateWorldRender { mem::transmute(0x787A2Cu32) }
pub unsafe fn ref_load_world() -> LoadWorld { mem::transmute(0x5D75C4u32) }
pub unsafe fn ref_send_packet() -> SendPacket { mem::transmute(0x404D50u32) }
pub unsafe fn ref_protocol_core() -> ProtocolCore { mem::transmute(0x62C170u32) }
pub unsafe fn ref_get_item_name() -> GetItemName { mem::transmute(0x5843E0u32) }
pub unsafe fn ref_print_screen() -> PrintScreen { mem::transmute(0x5DEBAFu32) }
pub unsafe fn ref_entity_action() -> EntityAction { mem::transmute(0x523F70u32) }
pub unsafe fn ref_entity_table() -> &'static mut [model::Entity] {
  let table_ref: *const *mut model::Entity = mem::transmute(0x79B9D40u32);
  slice::from_raw_parts_mut(*table_ref, 400)
}

pub unsafe fn ref_user() -> &'static mut model::User {
  let user_ref: *const *mut model::User = mem::transmute(0x79FAE5Cu32);
  (*user_ref).as_mut().expect("retrieving user object")
}

pub unsafe fn ref_character() -> &'static mut model::Character {
  let character_ref: *const *mut model::Character = mem::transmute(0x79FAE60u32);
  (*character_ref).as_mut().expect("retrieving character object")
}

pub unsafe fn ref_character_entity() -> &'static mut model::Entity {
	let character_ref: *mut *mut model::Entity = mem::transmute(0x79B9D48u32);
  (*character_ref).as_mut().expect("retrieving character entity")
}

pub unsafe fn ref_using_potion_type() -> &'static mut i32 { mem::transmute(0x7FE8E74u32) }
pub unsafe fn ref_item_loot_ranged() -> &'static mut bool { mem::transmute(0x7A8E0F4u32) }
pub unsafe fn ref_item_loot_request() -> &'static mut u32 { mem::transmute(0x854058u32) }
pub unsafe fn ref_item_loot_index() -> &'static mut u32 { mem::transmute(0x7A8E14Cu32) }
pub unsafe fn ref_loot_table() -> &'static mut [model::ItemEntity] {
  slice::from_raw_parts_mut(mem::transmute(0x7A8E352u32), 1000)
}
