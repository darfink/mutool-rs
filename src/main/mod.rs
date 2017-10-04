use std::{io, slice};
use std::path::Path;
use std::rc::Rc;
use std::os::raw::c_char;
use detour::{Detour, StaticDetour};
use muonline_packet::{Packet, PacketType};
use tap::TapResultOps;
use {toml, mu, ext, util, TOOL};

mod module;
mod notify;

pub struct MuTool {
  #[allow(dead_code)]
  proto: StaticDetour<ext::ProtocolCore>,
  #[allow(dead_code)]
  render: StaticDetour<ext::StateWorldRender>,
  #[allow(dead_code)]
  chat: StaticDetour<ext::ChatHandler>,
  modules: Vec<Box<module::Module>>,
}

impl MuTool {
  pub unsafe fn new() -> io::Result<Self> {
    let config = Self::load_config("mutool.toml")?;
    let proto = Self::init_proto(ext::ref_protocol_core())?;
    let render = Self::init_render(ext::ref_state_world_render())?;
    let chat = Self::init_chat(ext::ref_chat_handler())?;

    let pb_config = config.get("PushBullet")
      .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing 'PushBullet' config entry"))?
      .clone();
    let pushbullet = notify::Pushbullet::new(pb_config)?;
    let service = Rc::new(pushbullet);

    let builders = module::load_modules(&config, service);
    let modules = builders.into_iter()
      .filter_map(|builder| {
        let name = builder.name();
        builder
          .build()
          .tap_err(|error| eprintln!("[Tool:Error] Failed to initialize module {}: {}", name, error))
          .ok()
      }).collect::<Vec<_>>();

    println!("[Tool] Loaded modules:");
    for module in &modules {
      println!("[Tool] - {}", module.name());
    }

    Ok(MuTool { proto, render, chat, modules })
  }

  unsafe fn process(&mut self, packet: &Packet) {
    for module in &mut self.modules {
      module.process(packet);
    }
  }

  unsafe fn render(&mut self) {
    let renderer = ext::model::Renderer::get();
    for module in &mut self.modules {
      module.update();
      module.render(&renderer);
    }
  }

  unsafe fn chat(&mut self, text: &str) -> bool {
    self.modules.iter_mut().any(|module| module.chat(text))
  }

  unsafe fn load_config<P: AsRef<Path>>(path: P) -> io::Result<toml::Value> {
    util::read_file_contents(path)?.parse::<toml::Value>()
      .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
  }

  unsafe fn init_proto(target: ext::ProtocolCore)
      -> io::Result<StaticDetour<ext::ProtocolCore>> {
    let detour = |code, data, mut size, encrypted| {
      let result = DetourProtocolCore
        .get()
        .expect("calling original protocol core")
        .call(code, data, size, encrypted);

      if code as u8 == mu::protocol::realm::MagicAttackResult::CODE {
        *data.offset(1) = 9;
        size = 9;
      }

      let source = slice::from_raw_parts(data, size as usize);
      match Packet::from_bytes(source) {
        Err(error) => eprintln!("[Tool:Error] Failed to parse packet: {}", error),
        Ok(packet) => match TOOL.as_mut() {
          Some(tool) => tool.process(&packet),
          None => eprintln!("[Tool:Error] No active instance"),
        }
      }

      result
    };

    DetourProtocolCore.initialize(target, detour)
      .and_then(|mut hook| hook.enable().map(|_| hook))
      .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
  }

  unsafe fn init_render(target: ext::StateWorldRender)
      -> io::Result<StaticDetour<ext::StateWorldRender>> {
    let detour = || {
      DetourStateWorldRender
        .get()
        .expect("calling original world render")
        .call();

      match TOOL.as_mut() {
        Some(tool) => tool.render(),
        None => eprintln!("[Tool:Error] No active instance"),
      }
    };

    DetourStateWorldRender.initialize(target, detour)
      .and_then(|mut hook| hook.enable().map(|_| hook))
      .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
  }

  unsafe fn init_chat(target: ext::ChatHandler)
      -> io::Result<StaticDetour<ext::ChatHandler>> {
    let detour = |text, item| {
      match TOOL.as_mut() {
        Some(tool) => {
          let text = ::std::ffi::CStr::from_ptr(text).to_string_lossy();
          if tool.chat(&text) {
            return true;
          }
        },
        None => eprintln!("[Tool:Error] No active instance"),
      }

      DetourChatHandler
        .get()
        .expect("calling original chat handler")
        .call(text, item)
    };

    DetourChatHandler.initialize(target, detour)
      .and_then(|mut hook| hook.enable().map(|_| hook))
      .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
  }
}

static_detours! {
  struct DetourProtocolCore: extern "C" fn(u32, *mut u8, u32, bool) -> bool;
  struct DetourStateWorldRender: extern "C" fn();
  struct DetourChatHandler: extern "C" fn(*const c_char, bool) -> bool;
}