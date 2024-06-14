use super::super::provider::EmbeddingProvider;
use crate::{scheduler::job_name::JobName, settings::OpenAISettings};
use anyhow::Result;
use async_openai::{
  config::OpenAIConfig, error::OpenAIError, types::CreateEmbeddingRequestArgs, Client,
};
use async_trait::async_trait;
use governor::{DefaultDirectRateLimiter, Jitter, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero::nonzero;
use std::time::Duration;
use tracing::{error, info, warn};

lazy_static! {
  /**
   * API limit is 5000 req/min, 5,000,000 tokens/min.
   * Assuming:
   * - average batch is 100 inputs,
   * - average input is 400 words,
   * - average word is 5 characters,
   * - average token is 4 characters,
   *
   * Then:
   * - 100 * 400 * 5 = 200,000 characters, 200,000 / 4 = 50,000 tokens per request
   * - 5,000,000 / 50,000 = 100 requests/min
   * - 100 / 60 = 1.67 requests/sec
   */
  static ref RATE_LIMITER: DefaultDirectRateLimiter = RateLimiter::direct(Quota::per_second(nonzero!(1u32)));
}

pub struct OpenAIEmbeddingProvider {
  client: Client<OpenAIConfig>,
}

impl OpenAIEmbeddingProvider {
  pub fn new(settings: &OpenAISettings) -> Self {
    Self {
      client: Client::with_config(OpenAIConfig::default().with_api_key(&settings.api_key)),
    }
  }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
  fn name(&self) -> String {
    "openai-default".to_string()
  }

  fn dimensions(&self) -> usize {
    1024
  }

  fn batch_size(&self) -> usize {
    100
  }

  fn concurrency(&self) -> usize {
    1
  }

  fn interval(&self) -> Duration {
    Duration::from_secs(1)
  }

  fn job_name(&self) -> JobName {
    JobName::GenerateOpenAIEmbeddings
  }

  #[tracing::instrument(name = "OpenAIEmbeddingProvider::generate", skip_all, fields(count = payloads.len()))]
  async fn generate(&self, payloads: Vec<String>) -> Result<Vec<Vec<f32>>> {
    RATE_LIMITER
      .until_ready_with_jitter(Jitter::up_to(Duration::from_secs(1)))
      .await;

    let payload_char_count = payloads.iter().map(|s| s.len()).sum::<usize>();
    info!(
      "Generating embeddings for {} payloads with {} characters",
      payloads.len(),
      payload_char_count
    );

    if payload_char_count > 200000 {
      warn!("Payloads exceed 200,000 characters: {}", payload_char_count);
    }

    let request = CreateEmbeddingRequestArgs::default()
      .model("text-embedding-3-large")
      .input(payloads)
      .dimensions(self.dimensions() as u32)
      .build()?;
    let response = self
      .client
      .embeddings()
      .create(request)
      .await
      .inspect_err(|e| {
        if let OpenAIError::ApiError(err) = e {
          error!(
            "OpenAI API error: {} : {} : {} : {}",
            &err.code.as_ref().map(|s| s.to_string()).unwrap_or_default(),
            &err
              .param
              .as_ref()
              .map(|s| s.to_string())
              .unwrap_or_default(),
            &err.message,
            &err
              .r#type
              .as_ref()
              .map(|s| s.to_string())
              .unwrap_or_default()
          );
        }
      })?;
    let embeddings = response
      .data
      .into_iter()
      .map(|embedding| embedding.embedding)
      .collect::<Vec<Vec<f32>>>();
    Ok(embeddings)
  }
}
