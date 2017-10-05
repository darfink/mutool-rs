use std::{time, io, mem};
use std::collections::HashSet;
use serde::{Deserialize, Deserializer};
use muonline_packet::{Packet, PacketDecodable};
use detour::{StaticDetour, Detour};
use tap::{TapBooleanOps, TapOptionOps};
use {mu, ext};

/// The module's name.
pub const MODULE: &'static str = "UserStats";

/// An implementation of user stats.
struct UserStats {
  config: Config,
  session: Option<Session>,
  killed: HashSet<u16>,
}

impl UserStats  {
  /// Creates a new user stats module.
  unsafe fn new(config: Config) -> Self {
    UserStats {
      config,
      session: None,
      killed: HashSet::new(),
    }
  }

  /// Reports the session and ends it.
  unsafe fn report_and_end(&mut self) {
    if self.session.is_some() {
      self.report();
      self.session = None;
    }
  }

  /// Reports the current session's statistics.
  unsafe fn report(&self) {
    let session = try_opt!(self.session.as_ref()
      .tap_none(|| ext::func::show_notice("No session is currently active")));
    let seconds_passed = session.elapsed().as_secs() as f64;
    let hours_passed = seconds_passed / 3600f64;

    if self.config.experience && session.experience > 0 {
      let xp_of_level = Self::xp_level_percentage(session.start_xp, session.experience);
      let xp_of_level_per_hour = xp_of_level / hours_passed;

      ext::func::show_notice(
        format!("Experience (total / % / %h): {} / {:.1}% / {:.1}%h",
          session.experience,
          xp_of_level * 100f64,
          xp_of_level_per_hour * 100f64));
    }

    if self.config.kills && session.kills > 0 {
      let kills_per_hour = ((session.kills as f64) / hours_passed).round();
      ext::func::show_notice(
        format!("Kills (total / per hour): {} / {}", session.kills, kills_per_hour as u32));
    }

    if self.config.damage && session.damage > 0 {
      let damage_per_second = (session.damage as f64) / seconds_passed;
      ext::func::show_notice(
        format!("Damage (total / DPS): {} / {:.1}", session.damage, damage_per_second));
    }

    if self.config.money && session.money > 0 {
      let zen_per_hour = ((session.money as f64) / hours_passed).round();
      ext::func::show_notice(
        format!("Money (total / per hour): {} / {}", session.money, zen_per_hour as u64));
    }
  }

  /// Returns the amount of XP percentage received relative to each level.
  fn xp_level_percentage(mut xp_base: u64, mut xp_gained: u64) -> f64 {
    let mut xp_percentage = 0f64;
    let mut level = Self::xp_to_level(xp_base);

    while xp_gained > 0 {
      // Calculate the total amount XP required for the level
      let xp_level_total = Self::xp_for_level(level);

      // Calculate the XP difference between this level and the previous
      let xp_level_diff = xp_level_total - Self::xp_for_level(level - 1);

      // Determine how much of the XP was gained during this level
      let xp_on_level = xp_gained.min(xp_level_total - xp_base);

      xp_percentage += (xp_on_level as f64) / (xp_level_diff as f64);
      xp_gained -= xp_on_level;
      xp_base = xp_level_total;
      level += 1;
    }

    xp_percentage
  }

  /// Returns the level from a amount of XP.
  fn xp_to_level(xp: u64) -> u16 {
    let mut level = 1;
    while xp >= Self::xp_for_level(level) {
      level += 1;
    }
    level
  }

  /// Returns the amount of XP required for a level up.
  fn xp_for_level(level: u16) -> u64 {
    let level = level as u64;
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
    if let Ok(event) = mu::protocol::realm::EntityDeath::from_packet(packet) {
      if event.attacker_id == ext::ref_character_entity().id {
        self.killed.insert(event.victim_id);
      } else {
        self.killed.remove(&event.victim_id);
      }
    } else if let Ok(event) = mu::protocol::realm::PlayerExperienceExt::from_packet(packet) {
      let session = self.session.get_or_insert_with(|| {
        Session::new(ext::model::Character::get().experience as u64)
      });

      session.end_time = time::Instant::now();
      session.experience += event.experience() as u64;
      session.kills += 1;

      let victim_id = event.victim_id & 0x7FFF;
      if self.killed.remove(&victim_id) {
        println!("[DpsMeter] Damage (KILL): {}", event.damage);
        session.damage += event.damage as u64;
      }

      return;
    }

    let session = try_opt!(self.session.as_mut());

    if let Ok(attack) = mu::protocol::realm::PlayerDamageExt::from_packet(packet) {
      if attack.victim_id != ext::ref_character_entity().id {
        println!("[DpsMeter] Damage (HIT): {}", attack.damage);
        session.damage += attack.damage as u64;
      }
    } else if let Ok(event) = mu::protocol::realm::ItemGetResult::from_packet(packet) {
      if let mu::protocol::realm::ItemGetResult::Money(_) = event {
        let item = &ext::ref_loot_table()[*ext::ref_item_loot_index() as usize];
        if item.is_active && item.info.is_zen() {
          session.money += item.info.modifier as u64;
        }
      }
    }
  }

  /// Checks if the user want's the stats to be reported.
  unsafe fn chat(&mut self, text: &str) -> bool {
    (text == "/stats").tap_true(|_| self.report())
  }
}

#[derive(Debug)]
struct Session {
  start_time: time::Instant,
  end_time: time::Instant,
  start_xp: u64,
  experience: u64,
  damage: u64,
  kills: u64,
  money: u64,
}

impl Session {
  fn new(experience: u64) -> Self {
    Session {
      start_time: time::Instant::now(),
      end_time: time::Instant::now(),
      start_xp: experience,
      experience: 0,
      damage: 0,
      kills: 0,
      money: 0,
    }
  }

  fn elapsed(&self) -> time::Duration {
    let duration = self.end_time.duration_since(self.start_time);

    if duration.as_secs() < 3 {
      time::Duration::from_secs(3)
    } else {
      duration
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
  #[serde(default)]
  pub experience: bool,
  #[serde(default)]
  pub damage: bool,
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
      (*stats.0).report_and_end();
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
