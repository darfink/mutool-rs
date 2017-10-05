use std::io;
use std::ffi::CStr;
use muonline_packet::{self, Packet, PacketEncodable};
use mu;

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

pub unsafe fn send_packet(packet: &Packet, encrypted: bool) -> io::Result<()> {
  let data = packet.to_bytes_ex(Some(&muonline_packet::XOR_CIPHER), None);
  match super::ref_send_packet()(data.as_ptr(), data.len(), encrypted as u32, 0) {
    false => Err(io::Error::new(io::ErrorKind::Other, "Failed to send packet")),
    true => Ok(())
  }
}
