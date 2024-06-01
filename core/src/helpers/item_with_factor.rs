use crate::proto;

use super::math::desc_sort_by;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, Eq)]
pub struct ItemWithFactor {
  pub item: String,
  pub factor: u32,
}

impl Ord for ItemWithFactor {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.factor.cmp(&other.factor)
  }
}

impl PartialOrd for ItemWithFactor {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for ItemWithFactor {
  fn eq(&self, other: &Self) -> bool {
    self.factor == other.factor
  }
}

pub fn desc_sort_by_factor(values: &mut [ItemWithFactor]) {
  desc_sort_by(values, |item| item.factor as f32);
}

impl From<ItemWithFactor> for proto::ItemWithFactor {
  fn from(val: ItemWithFactor) -> Self {
    proto::ItemWithFactor {
      item: val.item,
      factor: val.factor,
    }
  }
}
