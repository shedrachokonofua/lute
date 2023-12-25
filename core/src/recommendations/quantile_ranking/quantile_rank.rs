use std::cmp::{Ord, Ordering};
use std::collections::BTreeMap;
use std::fmt::Debug;

pub struct QuantileRanking<T: Ord + Debug + Clone> {
  map: BTreeMap<T, usize>,
  total: usize,
}

impl<T: Ord + Debug + Clone> QuantileRanking<T> {
  pub fn new(data: &Vec<T>) -> Self {
    let mut map = BTreeMap::new();
    let total = data.len();
    for item in data {
      *map.entry(item.clone()).or_insert(0) += 1;
    }
    QuantileRanking { map, total }
  }

  pub fn get_rank(&self, key: &T) -> f64 {
    let mut rank_sum = 0;
    let mut count = 0;

    for (item, &cnt) in self.map.range(..=key) {
      match item.cmp(key) {
        Ordering::Less => rank_sum += cnt,
        Ordering::Equal => count = cnt,
        Ordering::Greater => break,
      }
    }

    let mid = (rank_sum as f64 + (rank_sum + count) as f64) / 2.0;
    mid / self.total as f64
  }
}
