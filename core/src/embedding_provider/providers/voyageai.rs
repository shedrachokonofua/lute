use crate::{
  embedding_provider::provider::EmbeddingProvider, scheduler::job_name::JobName,
  settings::VoyageAISettings,
};
use anyhow::Result;
use async_trait::async_trait;
use governor::{DefaultDirectRateLimiter, Jitter, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero::nonzero;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
  collections::HashMap,
  time::{Duration, Instant},
};
use tracing::info;

lazy_static! {
  /**
   * API limit is 300 req/min, 1,000,000 tokens/min.
   * Assuming:
   * - average batch is 100 inputs,
   * - average input is 400 words,
   * - average word is 5 characters,
   * - average token is 4 characters,
   *
   * Then:
   * - 128 * 400 * 5 = 256,000 characters, 256,000 / 4 = 64,000 tokens per request
   * - 1,000,000 / 64,000 = 15.625 requests/min
   * - 15.625 / 60 = 0.26 requests/sec or ~4 seconds per request
   */
  static ref RATE_LIMITER: DefaultDirectRateLimiter = RateLimiter::direct(Quota::per_minute(nonzero!(15u32)));
}

pub struct VoyageAIEmbeddingProvider {
  client: Client,
  settings: VoyageAISettings,
}

impl VoyageAIEmbeddingProvider {
  pub fn new(settings: VoyageAISettings) -> Self {
    Self {
      client: Client::new(),
      settings,
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct VoyageAIEmbeddingResponse {
  object: String,
  data: Vec<VoyageAIEmbedding>,
  model: String,
  usage: VoyageAIUsage,
  #[serde(flatten)]
  _unknown: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VoyageAIEmbedding {
  object: String,
  embedding: Vec<f32>,
  index: usize,
  #[serde(flatten)]
  _unknown: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VoyageAIUsage {
  total_tokens: usize,
  #[serde(flatten)]
  _unknown: HashMap<String, String>,
}

#[async_trait]
impl EmbeddingProvider for VoyageAIEmbeddingProvider {
  fn name(&self) -> String {
    "voyageai-default".to_string()
  }

  fn dimensions(&self) -> usize {
    1024
  }

  fn batch_size(&self) -> usize {
    128
  }

  fn concurrency(&self) -> usize {
    1
  }

  fn interval(&self) -> Duration {
    Duration::from_secs(4)
  }

  fn job_name(&self) -> JobName {
    JobName::GenerateVoyageAIEmbeddings
  }

  #[tracing::instrument(name = "VoyageAIEmbeddingProvider::generate", skip_all, fields(count = payloads.len()))]
  async fn generate(&self, payloads: Vec<String>) -> Result<Vec<Vec<f32>>> {
    RATE_LIMITER
      .until_ready_with_jitter(Jitter::up_to(Duration::from_secs(1)))
      .await;
    let start = Instant::now();
    let char_count = payloads.iter().map(|s| s.len()).sum::<usize>();
    let response = self
      .client
      .post("https://api.voyageai.com/v1/embeddings")
      .header("Authorization", format!("Bearer {}", self.settings.api_key))
      .json(&json!({
        "input": payloads,
        "model": "voyage-3-large",
        "input_type": "document"
      }))
      .send()
      .await?;
    let elapsed = start.elapsed();
    let body = response.json::<VoyageAIEmbeddingResponse>().await?;
    info!(
      elapsed = elapsed.as_millis(),
      char_count = char_count,
      token_count = body.usage.total_tokens,
      "VoyageAI embeddings generated"
    );
    Ok(body.data.into_iter().map(|data| data.embedding).collect())
  }
}
