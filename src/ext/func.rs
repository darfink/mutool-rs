use std::{io, ptr};
use std::ffi::CStr;
use muonline_packet::{self, Packet, PacketEncodable};
use mu;
use super::model;

pub unsafe fn get_item_name(code: mu::ItemCode, level: u8) -> String {
  let mut buffer = [0; 256];
  super::ref_get_item_name()(code.as_raw(), level, buffer.as_mut_ptr() as *mut _);

  // Item names may contain invalid unicode
  CStr::from_ptr(buffer.as_ptr())
  	.to_string_lossy()
  	.into()
}

pub unsafe fn show_notice<S: Into<String>>(message: S) {
  let packet = mu::protocol::realm::Message::Notice(message.into())
    .to_packet()
    .expect("converting message to packet")
    .to_bytes();
  super::ref_receive_notice()(packet.as_ptr());
}

/// Sends an item pickup request to the server.
pub unsafe fn pickup_item(item: &model::ItemEntity) {
  // Abort if there is a pending network request
  if *super::ref_item_loot_request() != u32::max_value() {
    return;
  }

  let index = super::ref_loot_table().iter()
    .position(|other| ptr::eq(other, item))
    .expect("retrieving item table index");

  *super::ref_item_loot_request() = index as u32;
  *super::ref_item_loot_index() = index as u32;

  mu::protocol::client::ItemGetRequest::new(index as u16).to_packet()
  .and_then(|packet| {
    super::func::send_packet(&packet, true)
  }).expect("sending item request packet");
}

pub unsafe fn send_packet(packet: &Packet, encrypted: bool) -> io::Result<()> {
  let data = packet.to_bytes_ex(Some(&muonline_packet::XOR_CIPHER), None);
  match super::ref_send_packet()(data.as_ptr(), data.len(), encrypted as u32, 0) {
  	false => Err(io::Error::new(io::ErrorKind::Other, "Failed to send packet")),
  	true => Ok(())
  }
}