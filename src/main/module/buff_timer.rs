use std::{io, time};
use std::collections::HashMap;
use serde::{Deserialize, Deserializer};
use num_traits::FromPrimitive;
use muonline_packet::{Packet, PacketDecodable};
use mu;
use ext::model;

/// The module's name.
pub const MODULE: &'static str = "BuffTimer";

/// An implementation of a buff timer.
struct BuffTimer {
  config: Config,
  buffs: HashMap<u16, HashMap<mu::Skill, Buff>>,
}

impl BuffTimer {
  /// Creates a new buff timer instance.
  fn new(config: Config) -> Self {
    BuffTimer { config, buffs: HashMap::new() }
  }
}

impl super::Module for BuffTimer {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Analyzes a packet to detect if a buff is applied.
  unsafe fn process(&mut self, packet: &Packet) {
    if let Ok(event) = mu::protocol::realm::MagicAttackResult::from_packet(packet) {
      let skill = try_opt!(mu::Skill::from_u16(event.skill));
      let buff = try_opt!(BUFFS.get(&skill));

      let source = model::Entity::from_id(event.source_id).expect("retrieving source entity");
      let target = model::Entity::from_id(event.target_id & 0x7FFF).expect("retrieving target entity");

      let character_id = model::Entity::character().id;
      let character_is_source = source.id == character_id;

      // If the user is the source or target the buff is of interest
      if character_is_source || target.id == character_id {
        let duration = match buff.duration {
          // Static durations encompasses damage and defense buffs
          BuffDuration::Static(duration) => duration,
          // The duration of dynamic's buffs can only be determined if the user
          // is the source, since character stats need to be taken into account.
          BuffDuration::Dynamic(duration) if character_is_source => {
            duration(model::Character::get())
          },
          _ => return,
        };

        // Insert the buff to the target's active list
        self.buffs.entry(target.id)
          .or_insert_with(HashMap::new)
          .insert(skill, Buff::new(buff.name, duration));
      } else {
        // TODO: Remove buffs if replaced by other users
      }
    }
  }

  /// Removes expired entities and buffs.
  unsafe fn update(&mut self) {
    // Iterate over each user that is tracked
    self.buffs.retain(|id, buffs| {
      model::Entity::from_id(*id).iter()
        // Only retain the buff is the target is active
        .filter(|entity| entity.is_active)
        .map(|_| {
          // Remove all buffs that have expired
          buffs.retain(|_, buff| buff.time_left() > 0f32);
          !buffs.is_empty()
        })
        .next()
        .unwrap_or(false)
    });
  }

  /// Renders the graphical elements of the buff timer.
  unsafe fn render(&mut self, renderer: &model::Renderer) {
    const POS_X: f32 = 549.6;
    const POS_Y: f32 = 428.8;
    const PADDING: f32 = 4.0;
    const BUFF_PADDING: f32 = 0.8;
    const BUFF_WIDTH: f32 = 82.4;
    const BUFF_HEIGHT: f32 = 6.4;
    const NAME_HEIGHT: f32 = 8.0;

    let mut position_y = POS_Y;

    for (id, buffs) in &self.buffs {
      let user = match model::Entity::from_id(*id) {
        Some(entity) if entity.is_active => entity,
        _ => continue,
      };

      let bg_height = NAME_HEIGHT + BUFF_HEIGHT * (buffs.len() as f32) + PADDING * (2.0 + buffs.len() as f32);
      position_y -= bg_height;

      let mut offset_y = position_y;
      let offset_x = POS_X + PADDING;

      // Draw the shaded background
      renderer.draw_rectangle(
        POS_X,
        offset_y,
        BUFF_WIDTH + PADDING * 2.0,
        bg_height,
        mu::Color::BLACK.alpha(0x99));

      // Everything but the background is affected by the padding
      offset_y += PADDING;

      // Draw the user's name in a unique color
      renderer.draw_text(
        user.name(),
        offset_x as i32,
        offset_y as i32,
        mu::Color::from_str(user.name()),
        mu::Color::TRANSPARENT);

      // Update the offset for the adjacent buff
      offset_y += NAME_HEIGHT + PADDING;

      for (_, buff) in buffs {
        let time_left = buff.time_left();
        let time_left_percent = time_left / (buff.duration.as_secs() as f32);
        let time_left_secs = time_left.round() as u64;

        let should_warn = time_left_secs < self.config.warn;
        let alpha = if should_warn && time_left_secs % 2 == 0 { 0 } else { 1 };

        // Draw the buff border
        renderer.draw_rectangle(
          offset_x,
          offset_y,
          BUFF_WIDTH,
          BUFF_HEIGHT,
          mu::Color::from_hex(0x006000).alpha(alpha * 0x7F));

        let buff_width = BUFF_WIDTH - BUFF_PADDING * 2.0;
        let buff_height = BUFF_HEIGHT - BUFF_PADDING * 2.0;

        // Draw the buff background
        renderer.draw_rectangle(
          offset_x + BUFF_PADDING,
          offset_y + BUFF_PADDING,
          buff_width,
          buff_height,
          mu::Color::from_hex(0x008000).alpha(alpha * 0xB2));

        // Draw the buff timer
        renderer.draw_rectangle(
          offset_x + BUFF_PADDING,
          offset_y + BUFF_PADDING,
          buff_width * time_left_percent,
          buff_height,
          mu::Color::from_hex(0x00C000).alpha(alpha * 0xFF));

        // Update the offset for the next buff
        offset_y += BUFF_HEIGHT + PADDING;
      }
    }
  }
}

/// A representation of a buff event.
struct Buff {
  // TODO: Replace with color?
  name: &'static str,
  time: time::Instant,
  duration: time::Duration,
}

impl Buff {
  /// Creates a new buff with the current instant as start time.
  pub fn new(name: &'static str, duration: time::Duration) -> Self {
    Buff { name, duration, time: time::Instant::now() }
  }

  /// Returns the number of seconds left of the buff.
  pub fn time_left(&self) -> f32 {
    self.duration
      .checked_sub(self.time.elapsed())
      .map(Self::duration_to_float)
      .unwrap_or(0f32)
  }

  /// Converts a duration to a float representation.
  fn duration_to_float(duration: time::Duration) -> f32 {
    (duration.as_secs() as f32) +
    (duration.subsec_nanos() as f32 / 1_000_000_000f32)
  }
}

/// A description of buff durations.
enum BuffDuration {
  Static(time::Duration),
  Dynamic(fn(&model::Character) -> time::Duration),
}

/// A description of a buff.
struct BuffMeta {
  name: &'static str,
  duration: BuffDuration,
}

lazy_static! {
  /// A collection of meta information for buffs.
  static ref BUFFS: HashMap<mu::Skill, BuffMeta> = {
    let mut buffs = HashMap::new();

    buffs.insert(mu::Skill::Defense, BuffMeta {
      name: "Defense",
      duration: BuffDuration::Static(time::Duration::from_secs(60)),
    });

    buffs.insert(mu::Skill::Attack, BuffMeta {
      name: "Damage",
      duration: BuffDuration::Static(time::Duration::from_secs(60)),
    });

    buffs.insert(mu::Skill::KnightAddLife, BuffMeta {
      name: "Greater Fortitude",
      duration: BuffDuration::Dynamic(|character| {
        time::Duration::from_secs(60 + character.energy as u64 / 10)
      }),
    });

    buffs.insert(mu::Skill::Magicdefense, BuffMeta {
      name: "Soul Barrier",
      duration: BuffDuration::Dynamic(|character| {
        time::Duration::from_secs(60 + character.energy as u64 / 40)
      }),
    });

    buffs
  };
}

/// Configuration for `BuffTimer`.
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
  #[serde(default)]
  pub enabled: bool,
  #[serde(default)]
  pub warn: u64,
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
}

impl super::ModuleBuilder for Builder {
  /// Returns the module's name.
  fn name(&self) -> &'static str { MODULE }

  /// Returns whether this module is enabled or not.
  fn enabled(&self) -> bool { self.config.enabled }

  /// Builds the module associated with this builder.
  unsafe fn build(self: Box<Self>) -> io::Result<Box<super::Module>> {
    let module = Box::new(BuffTimer::new(self.config));
    Ok(module as Box<_>)
  }
}