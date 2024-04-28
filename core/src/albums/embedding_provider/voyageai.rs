use super::{helpers::get_embedding_api_input, provider::AlbumEmbeddingProvider};
use crate::{
  albums::album_read_model::AlbumReadModel, helpers::key_value_store::KeyValueStore,
  settings::VoyageAISettings,
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration;
use core::time;
use governor::{DefaultDirectRateLimiter, Jitter, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero::nonzero;
use reqwest::Client;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::error;

lazy_static! {
  static ref RATE_LIMITER: DefaultDirectRateLimiter = RateLimiter::direct(Quota::per_second(nonzero!(4u32))); // API limit is 300/min
}

pub struct VoyageAIAlbumEmbeddingProvider {
  kv: Arc<KeyValueStore>,
  client: Client,
  settings: VoyageAISettings,
}

impl VoyageAIAlbumEmbeddingProvider {
  pub fn new(settings: &VoyageAISettings, kv: Arc<KeyValueStore>) -> Self {
    Self {
      kv,
      client: Client::new(),
      settings: settings.clone(),
    }
  }
}

fn document_kv_key(id: &str) -> String {
  format!("voyageai:doc:{}", id)
}

#[async_trait]
impl AlbumEmbeddingProvider for VoyageAIAlbumEmbeddingProvider {
  fn name(&self) -> &str {
    "voyageai-default"
  }

  fn dimensions(&self) -> usize {
    1536
  }

  fn concurrency(&self) -> usize {
    3
  }

  #[tracing::instrument(name = "VoyageAIAlbumEmbeddingProvider::generate", skip(self))]
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<f32>> {
    let (id, input) = get_embedding_api_input(album);
    if let Some(embedding) = self.kv.get::<Vec<f32>>(&document_kv_key(&id)).await? {
      return Ok(embedding);
    }

    let mut body = HashMap::new();
    body.insert("input", input);
    body.insert("model", "voyage-large-2".to_string());

    let _ = &RATE_LIMITER
      .until_ready_with_jitter(Jitter::up_to(time::Duration::from_secs(4)))
      .await;
    let res = self
      .client
      .post("https://api.voyageai.com/v1/embeddings")
      .header("Authorization", format!("Bearer {}", self.settings.api_key))
      .json(&body)
      .send()
      .await?;

    if !res.status().is_success() {
      return Err(anyhow::anyhow!(format!(
        "Failed to get embedding: {}",
        res.text().await?
      )));
    }
    let response: Value = res.json().await.map_err(|e| {
      error!("Failed to parse response: {}", e);
      anyhow::anyhow!(format!("Failed to parse response: {}", e))
    })?;
    let embedding: Vec<f32> = response["data"][0]["embedding"]
      .as_array()
      .unwrap()
      .iter()
      .map(|v| v.as_f64().unwrap() as f32)
      .collect();

    self
      .kv
      .set(&document_kv_key(&id), &embedding, Duration::try_weeks(8))
      .await?;
    Ok(embedding)
  }
}
