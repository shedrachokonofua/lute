use crate::files::file_metadata::file_name::FileName;
use serde::{Deserialize, Serialize};

pub fn average_embedding(embeddings: Vec<(&Vec<f32>, u32)>) -> Vec<f32> {
  let mut len = 0;
  let mut average_embedding = vec![0.0; embeddings[0].0.len()];
  for (embedding, weight) in embeddings {
    for (i, value) in embedding.iter().enumerate() {
      average_embedding[i] += value * weight as f32;
      len += weight;
    }
  }

  for value in average_embedding.iter_mut() {
    *value /= len as f32;
  }

  average_embedding
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct EmbeddingDocument {
  pub file_name: FileName,
  pub key: String,
  pub embedding: Vec<f32>,
}

pub fn embedding_to_bytes(embedding: &Vec<f32>) -> Vec<u8> {
  embedding
    .iter()
    .flat_map(|f| f.to_ne_bytes().to_vec())
    .collect()
}

impl EmbeddingDocument {
  pub fn embedding_bytes(&self) -> Vec<u8> {
    embedding_to_bytes(&self.embedding)
  }
}
