use std::io;
use std::rc::Rc;
use muonline_packet::Packet;
use main::notify::NotificationService;
use tap::{TapResultOps, TapOptionOps};
use {toml, ext};

mod auto_health_potion;
mod auto_repair;
mod buff_timer;
mod death_notifier;
mod loot_filter;
mod loot_notifier;
mod user_stats;

pub trait Module {
  /// Returns the module's name.
  fn name(&self) -> &'static str;

  /// Processes an incoming server packet.
  unsafe fn process(&mut self, packet: &Packet);

  /// Updates any module logic.
  unsafe fn update(&mut self) { }

  /// Renders any module elements.
  unsafe fn render(&mut self, _: &ext::model::Renderer) { }

  /// Processes any chat command
  unsafe fn chat(&mut self, _: &str) -> bool { false }
}

pub trait ModuleBuilder {
  /// Returns the module's name.
  fn name(&self) -> &'static str;

  /// Returns whether this module is enabled or not.
  fn enabled(&self) -> bool;

  /// Builds the module associated with this builder.
  unsafe fn build(self: Box<Self>) -> io::Result<Box<Module>>;
}

pub fn load_modules(config: &toml::Value, service: Rc<NotificationService>) -> Vec<Box<ModuleBuilder>> {
  let service2 = service.clone();
  let entries: Vec<(&'static str, Box<Fn(toml::Value) -> io::Result<Box<ModuleBuilder>>>)> = vec![
    (auto_health_potion::MODULE, Box::new(|config| {
      auto_health_potion::Builder::new(config)
        .map(|builder| Box::new(builder) as Box<ModuleBuilder>)
    })),
    (auto_repair::MODULE, Box::new(|config| {
      auto_repair::Builder::new(config)
        .map(|builder| Box::new(builder) as Box<ModuleBuilder>)
    })),
    (buff_timer::MODULE, Box::new(|config| {
      buff_timer::Builder::new(config)
        .map(|builder| Box::new(builder) as Box<ModuleBuilder>)
    })),
    (death_notifier::MODULE, Box::new(move |config| {
      death_notifier::Builder::new(config, service.clone())
        .map(|builder| Box::new(builder) as Box<ModuleBuilder>)
    })),
    (loot_filter::MODULE, Box::new(|config| {
      loot_filter::Builder::new(config)
        .map(|builder| Box::new(builder) as Box<ModuleBuilder>)
    })),
    (loot_notifier::MODULE, Box::new(move |config| {
      loot_notifier::Builder::new(config, service2.clone())
        .map(|builder| Box::new(builder) as Box<ModuleBuilder>)
    })),
    (user_stats::MODULE, Box::new(|config| {
      user_stats::Builder::new(config)
        .map(|builder| Box::new(builder) as Box<ModuleBuilder>)
    })),
  ];

  entries.into_iter()
    .filter_map(|(name, constructor)| {
      config.get(name)
        .tap_none(|| eprintln!("[Tool:Warning] Missing module config for {}", name))
        .and_then(|config| constructor(config.clone())
          .tap_err(|error| eprintln!("[Tool:Error] Failed to load module {}: {}", name, error))
          .ok())
    })
    .filter(|builder| builder.enabled())
    .collect::<Vec<_>>()
}