use std::io;
use std::rc::Rc;
use muonline_packet::Packet;
use main::notify::NotificationService;
use tap::TapResultOps;
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
}

pub trait ModuleBuilder {
  /// Returns the module's name.
  fn name(&self) -> &'static str;

  /// Returns whether this module is enabled or not.
  fn enabled(&self) -> bool;

  /// Builds the module associated with this builder.
  unsafe fn build(self: Box<Self>) -> io::Result<Box<Module>>;
}

pub fn init_modules(config: &toml::Value, service: Rc<NotificationService>) -> Vec<Box<ModuleBuilder>> {
  let results = vec![
    auto_health_potion::Builder::new(config[auto_health_potion::MODULE].clone())
      .map(|builder| Box::new(builder) as Box<ModuleBuilder>),
    auto_repair::Builder::new(config[auto_repair::MODULE].clone())
      .map(|builder| Box::new(builder) as Box<ModuleBuilder>),
    buff_timer::Builder::new(config[buff_timer::MODULE].clone())
      .map(|builder| Box::new(builder) as Box<ModuleBuilder>),
    death_notifier::Builder::new(config[death_notifier::MODULE].clone(), service.clone())
      .map(|builder| Box::new(builder) as Box<ModuleBuilder>),
    loot_filter::Builder::new(config[loot_filter::MODULE].clone())
      .map(|builder| Box::new(builder) as Box<ModuleBuilder>),
    loot_notifier::Builder::new(config[loot_notifier::MODULE].clone(), service.clone())
      .map(|builder| Box::new(builder) as Box<ModuleBuilder>),
    user_stats::Builder::new(config[user_stats::MODULE].clone())
      .map(|builder| Box::new(builder) as Box<ModuleBuilder>),
  ];

  results.into_iter()
    .filter_map(|result| {
      // TODO: Add module name to error handling
      result
        .tap_err(|error| eprintln!("[Tool:Error] Failed to load module: {}", error))
        .ok()
    }).collect()
}