use super::{helpers::get_embedding_api_input, provider::AlbumEmbeddingProvider};
use crate::{
  albums::album_read_model::AlbumReadModel,
  helpers::{
    batch_loader::{BatchLoader, BatchLoaderConfig, Loader, LoaderError},
    key_value_store::KeyValueStore,
  },
  settings::{Settings, VoyageAISettings},
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration as ChronoDuration;
use governor::{DefaultDirectRateLimiter, Jitter, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero::nonzero;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
  sync::Arc,
  time::{Duration, Instant},
};
use tracing::{error, info};

struct VoyageAILoader {
  client: Client,
  settings: Option<VoyageAISettings>,
}

#[async_trait]
impl Loader for VoyageAILoader {
  type Key = String;
  type Value = Vec<f32>;

  #[tracing::instrument(name = "VoyageAILoader::load", skip_all, fields(keys = keys.len()))]
  async fn load(&self, keys: &[String]) -> Vec<Result<Vec<f32>, LoaderError>> {
    RATE_LIMITER
      .until_ready_with_jitter(Jitter::up_to(Duration::from_secs(1)))
      .await;
    let start = Instant::now();
    let res = self
      .client
      .post("https://api.voyageai.com/v1/embeddings")
      .header(
        "Authorization",
        format!("Bearer {}", self.settings.clone().unwrap().api_key),
      )
      .json(&json!({
        "input": keys,
        "model": "voyage-large-2"
      }))
      .send()
      .await
      .map_err(|e| {
        error!("Failed to get embedding: {}", e);
        LoaderError {
          msg: format!("Failed to get embedding: {}", e),
        }
      });
    let elapsed = start.elapsed();
    info!("VoyageAI request took {:?}", elapsed);

    match res {
      Err(e) => return vec![Err(e); keys.len().clone()],
      Ok(res) => {
        if !res.status().is_success() {
          return vec![
            Err(LoaderError {
              msg: format!("Failed to get embedding: {}", res.text().await.unwrap()),
            });
            keys.len()
          ];
        }

        match res.json::<Value>().await.map_err(|e| {
          error!("Failed to parse response: {}", e);
          LoaderError {
            msg: format!("Failed to parse response: {}", e),
          }
        }) {
          Err(e) => return vec![Err(e); keys.len().clone()],
          Ok(response) => {
            let embeddings: Vec<Vec<f32>> = response["data"]
              .as_array()
              .unwrap()
              .iter()
              .map(|v| {
                v["embedding"]
                  .as_array()
                  .unwrap()
                  .iter()
                  .map(|v| v.as_f64().unwrap() as f32)
                  .collect()
              })
              .collect();

            return embeddings.into_iter().map(|e| Ok(e)).collect();
          }
        }
      }
    }
  }
}

lazy_static! {
  static ref RATE_LIMITER: DefaultDirectRateLimiter = RateLimiter::direct(Quota::per_second(nonzero!(4u32))); // API limit is 300/min
  static ref BATCH_LOADER: BatchLoader<VoyageAILoader> = BatchLoader::new(
    VoyageAILoader {
      client: Client::new(),
      settings: Settings::new().unwrap().embedding_provider.voyageai,
    },
    BatchLoaderConfig {
      batch_size: 128,
      time_limit: Duration::from_millis(250),
    }
  );
}

pub struct VoyageAIAlbumEmbeddingProvider {
  kv: Arc<KeyValueStore>,
}

impl VoyageAIAlbumEmbeddingProvider {
  pub fn new(kv: Arc<KeyValueStore>) -> Self {
    Self { kv }
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
    500
  }

  #[tracing::instrument(name = "VoyageAIAlbumEmbeddingProvider::generate", skip(self))]
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<f32>> {
    let (id, input) = get_embedding_api_input(album);
    if let Some(embedding) = self.kv.get::<Vec<f32>>(&document_kv_key(&id)).await? {
      return Ok(embedding);
    }

    let embedding = BATCH_LOADER.load(input).await?;

    self
      .kv
      .set(
        &document_kv_key(&id),
        &embedding,
        ChronoDuration::try_weeks(6)
          .map(|d| d.to_std())
          .transpose()?,
      )
      .await?;
    Ok(embedding)
  }
}
