use num_traits::{float::FloatCore, Num};

pub fn median(vals: Vec<f32>) -> f32 {
  let mut sorted_values = vals.clone();
  sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
  let count = sorted_values.len();
  if count == 0 {
    return 0.0;
  }

  let middle = count / 2;

  if count % 2 == 0 {
    (sorted_values[middle - 1] + sorted_values[middle]) / 2.0
  } else {
    sorted_values[middle]
  }
}

pub fn desc_sort_by<T, F>(values: &mut [T], f: F)
where
  F: Fn(&T) -> f32,
  T: Clone + Ord,
{
  values.sort_by(|a, b| f(b).partial_cmp(&f(a)).unwrap());
}

pub fn default_if_zero<T: Num + FloatCore>(value: T, default: T) -> T {
  if value.is_zero() || value.is_nan() {
    default
  } else {
    value
  }
}
