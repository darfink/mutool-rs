use std::{time, io, mem};
use serde::{Deserialize, Deserializer};
use muonline_packet::{Packet, PacketDecodable};
use detour::{StaticDetour, Detour};
use {mu, ext};

/// The module's name.
pub const MODULE: &'static str = "UserStats";

#[derive(Debug, Default)]
struct Session {
  experience: u32,
  kills: u32,
  money: u32,
}

/// An implementation of user stats.
struct UserStats {
  session: Option<(time::Instant, Session)>,
  config: Config,
}

impl UserStats  {
  /// Creates a new user stats module.
  unsafe fn new(config: Config) -> Self {
    UserStats { config, session: None }
  }

  /// Ends the current session, printing out all stats.
  unsafe fn end_session(&mut self) {
    if let Some((start_time, stats)) = self.session.take() {
      let hours_passed = (start_time.elapsed().as_secs() as f32) / 3600f32;

      if self.config.experience && stats.experience > 0 {
        let character = ext::model::Character::get();
        let xp_base = Self::xp_for_level(character.level - 1);
        let xp_for_level = character.experience_next - xp_base;

        let xp_of_level = (stats.experience as f32) / (xp_for_level as f32);
        let xp_of_level_per_hour = xp_of_level / hours_passed;

        ext::func::show_notice(
          format!("Experience (total / % / %H): {} / {:.1}% / {:.1}%H",
            stats.experience,
            xp_of_level * 100f32,
            xp_of_level_per_hour * 100f32));
      }

      if self.config.kills && stats.kills > 0 {
        let kills_per_hour = ((stats.kills as f32) / hours_passed).round();
        ext::func::show_notice(
          format!("Kills (total / per hour): {} / {}", stats.kills, kills_per_hour as u32));
      }

      if self.config.money && stats.money > 0 {
        let zen_per_hour = ((stats.money as f32) / hours_passed).round();
        ext::func::show_notice(
          format!("Money (total / per hour): {} / {}", stats.money, zen_per_hour as u32));
      }
    }
  }

  /// Returns the amount of XP required for a level up.
  fn xp_for_level(level: u16) -> u32 {
    let level = level as u32;
    if level == 0 { return 0; }
    let mut experience = 10 * level.pow(2) * (level + 9);

    if level > 255 {
      let base = level - 256;
      experience += 1000 * base.pow(2) * (base + 9)
    }
    experience
  }
}

impl super::Module for UserStats {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Analyzes a packet to track items of interest.
  unsafe fn process(&mut self, packet: &Packet) {
    if let Ok(event) = mu::protocol::realm::PlayerExperienceExt::from_packet(packet) {
      let &mut (_, ref mut session) = self.session
        .get_or_insert_with(|| (time::Instant::now(), Session::default()));

      session.experience += event.experience();
      session.kills += 1;
    }

    /*if let Ok(result) = mu::protocol::realm::ItemGetResult::from_packet(packet) {
      if let mu::protocol::realm::ItemGetResult::Money(money) = result {
        println!("[UserStats] Received {}", money);
        session.money += money;
      }
    }*/
  }
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
  #[serde(default)]
  pub experience: bool,
  #[serde(default)]
  pub kills: bool,
  #[serde(default)]
  pub money: bool,
}

static_detours! {
  struct DetourLoadWorld: extern "C" fn(u32);
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

  /// Initializes the detour used for loading worlds.
  unsafe fn init_hook(
      stats: *mut UserStats,
      target: ext::LoadWorld)
      -> io::Result<StaticDetour<ext::LoadWorld>> {
    let stats = ::util::SendPointer(stats);
    DetourLoadWorld.initialize(target, move |world| {
      DetourLoadWorld
        .get()
        .expect("retrieving detour controller")
        .call(world);
      (*stats.0).end_session();
    })
    .and_then(|mut hook| hook.enable().map(|_| hook))
    .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
  }
}

impl super::ModuleBuilder for Builder {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Returns whether this module is enabled or not.
  fn enabled(&self) -> bool {
    self.config.experience || self.config.kills || self.config.money
  }

  /// Builds the module associated with this builder.
  unsafe fn build(self: Box<Self>) -> io::Result<Box<super::Module>> {
    // TODO: How to handle detours in case the module is disabled?
    let mut module = Box::new(UserStats::new(self.config));
    mem::forget(Self::init_hook(&mut *module, ext::ref_load_world())?);
    Ok(module as Box<_>)
  }
}
