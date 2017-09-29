use std::cmp::Ordering;
use serde::{self, Deserialize, Deserializer};
use pest::Parser;
use strsim::levenshtein;
use {mu, ext};
use self::itemdb::ITEMDB;

mod itemdb;

#[cfg(debug_assertions)]
const _GRAMMAR: &'static str = include_str!("item.pest");

#[derive(Parser)]
#[grammar = "filter/item.pest"]
struct ItemParser;

#[derive(Debug)]
struct OrdFilter<T: Ord + Copy> {
  comparisons: Vec<Ordering>,
  value: T,
}

impl<T: Ord + Copy> OrdFilter<T> {
  pub fn new(value: T) -> Self {
    OrdFilter { comparisons: vec![Ordering::Equal], value }
  }

  pub fn apply(&self, other: &T) -> bool {
    self.comparisons.contains(&other.cmp(&self.value))
  }
}

#[derive(Default, Debug)]
pub struct ItemFilter {
  excellent: Option<bool>,
  code: Option<mu::ItemCode>,
  level: Option<OrdFilter<u8>>,
  option: Option<OrdFilter<u8>>,
  skill: Option<bool>,
  luck: Option<bool>,
}

impl ItemFilter {
  pub fn apply(&self, item: &ext::model::Item) -> bool {
    self.excellent.map(|f| f == item.is_excellent()).unwrap_or(true) &&
    self.code.map(|f| f == item.code()).unwrap_or(true) &&
    self.level.as_ref().map(|f| f.apply(&item.level())).unwrap_or(true) &&
    self.option.as_ref().map(|f| f.apply(&item.option())).unwrap_or(true) &&
    self.skill.map(|f| f == item.has_skill()).unwrap_or(true) &&
    self.luck.map(|f| f == item.has_luck()).unwrap_or(true)
  }
}

impl<'de> Deserialize<'de> for ItemFilter {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
    let source: String = Deserialize::deserialize(deserializer)?;

    let mut pairs = ItemParser::parse_str(Rule::item, &source)
      .map_err(|error| serde::de::Error::custom(error))?;
    let tokens = pairs.next().map(|p| p.into_inner())
      .ok_or_else(|| serde::de::Error::custom("empty item filter"))?;

    let mut filter = ItemFilter::default();

    for token in tokens {
      match token.as_rule() {
        Rule::excellent => filter.excellent = Some(true),
        Rule::skill => filter.skill = Some(true),
        Rule::luck => filter.luck = Some(true),
        Rule::name => {
          let name = token.as_str();
          let item = ITEMDB.iter()
            .find(|item| levenshtein(name, &item.name) <= 2)
            .ok_or_else(|| serde::de::Error::invalid_value(
              serde::de::Unexpected::Str(&name),
              &"a valid item name"))?;

          filter.code = Some(item.code);
          filter.level = item.level.map(OrdFilter::new);
        },
        Rule::level => {
          if filter.level.is_some() {
            return Err(serde::de::Error::custom("level is not applicable for this item"));
          }

          let mut pairs = token.into_inner();
          let comparisons = pairs
            .find(|p| p.as_rule() == Rule::comparator)
            .map(|p| {
              match p.as_str() {
                ">"  => vec![Ordering::Greater],
                ">=" => vec![Ordering::Equal, Ordering::Greater],
                "<=" => vec![Ordering::Equal, Ordering::Less],
                "<"  => vec![Ordering::Less],
                _ => unreachable!("invalid comparator"),
              }
            })
            .unwrap_or_else(|| vec![Ordering::Equal]);

          let value = pairs
            .find(|p| p.as_rule() == Rule::digits)
            .and_then(|p| u8::from_str_radix(p.as_str(), 10).ok())
            .expect("parser failed to enforce numbers");

          filter.level = Some(OrdFilter { value, comparisons });
        },
        Rule::option => {
          let mut pairs = token.into_inner();
          let comparisons = pairs
            .find(|p| p.as_rule() == Rule::comparator)
            .map(|p| {
              match p.as_str() {
                ">"  => vec![Ordering::Greater],
                ">=" => vec![Ordering::Greater, Ordering::Equal],
                "<=" => vec![Ordering::Less, Ordering::Equal],
                "<"  => vec![Ordering::Less],
                _ => unreachable!("contact developer"),
              }
            })
            .unwrap_or_else(|| vec![Ordering::Equal]);

          let mut value = pairs
            .find(|p| p.as_rule() == Rule::digits)
            .and_then(|p| u8::from_str_radix(p.as_str(), 10).ok())
            .expect("parser failed to enforce numbers");
          if value % 4 == 0 {
            value /= 4;
          } else if value % 5 == 0 {
            value /= 5;
          }

          filter.option = Some(OrdFilter { value, comparisons });
        },
        _ => return Err(serde::de::Error::custom("unexpected rule in item filter")),
      }
    }

    Ok(filter)
  }
}
