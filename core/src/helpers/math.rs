use num_traits::Num;

pub fn median_by<T, F>(sorted_values: &mut Vec<T>, f: F) -> f32
where
  F: Fn(&T) -> f32,
  T: Clone,
{
  let count = sorted_values.len();
  if count == 0 {
    return 0.0;
  }

  let middle = count / 2;

  if count % 2 == 0 {
    (f(&sorted_values[middle - 1]) + f(&sorted_values[middle])) / 2.0
  } else {
    f(&sorted_values[middle])
  }
}

pub fn desc_sort_by<T, F>(values: &mut [T], f: F)
where
  F: Fn(&T) -> f32,
  T: Clone + Ord,
{
  values.sort_by(|a, b| f(b).partial_cmp(&f(a)).unwrap());
}

pub fn default_if_zero<T: Num>(value: T, default: T) -> T {
  if value.is_zero() {
    default
  } else {
    value
  }
}
