use crate::{albums::album_read_model::AlbumReadModel, helpers::key_value_store::KeyValueStore};
use anyhow::Result;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct EmbeddingProviderCache {
  provider_name: String,
  pub kv: Arc<KeyValueStore>,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingProviderCacheItem {
  hash: String,
  embedding: Vec<f32>,
}

impl EmbeddingProviderCache {
  pub fn new(provider_name: &str, kv: Arc<KeyValueStore>) -> Self {
    Self {
      provider_name: provider_name.to_string(),
      kv,
    }
  }

  pub fn build_key(&self, album: &AlbumReadModel) -> String {
    format!(
      "embedding_provider_cache:{}:{}",
      self.provider_name,
      album.file_name.to_string()
    )
  }

  pub async fn set(&self, album: &AlbumReadModel, hash: String, embedding: Vec<f32>) -> Result<()> {
    self
      .kv
      .set::<EmbeddingProviderCacheItem>(
        self.build_key(album).as_str(),
        EmbeddingProviderCacheItem { hash, embedding },
        Duration::try_weeks(6).map(|d| d.to_std()).transpose()?,
      )
      .await?;
    Ok(())
  }

  pub async fn get(&self, album: &AlbumReadModel, hash: String) -> Result<Option<Vec<f32>>> {
    match self
      .kv
      .get::<EmbeddingProviderCacheItem>(self.build_key(album).as_str())
      .await?
    {
      Some(item) if item.hash == hash => Ok(Some(item.embedding)),
      _ => Ok(None),
    }
  }
}
