use num_traits::{float::FloatCore, Num};

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

pub fn default_if_zero<T: Num + FloatCore>(value: T, default: T) -> T {
  if value.is_zero() || value.is_nan() {
    default
  } else {
    value
  }
}

pub fn average_embedding(embeddings: Vec<&Vec<f32>>) -> Vec<f32> {
  let len = embeddings.len();
  let mut average_embedding = vec![0.0; embeddings[0].len()];
  for embedding in embeddings {
    for (i, value) in embedding.iter().enumerate() {
      average_embedding[i] += value;
    }
  }

  for value in average_embedding.iter_mut() {
    *value /= len as f32;
  }

  average_embedding
}
