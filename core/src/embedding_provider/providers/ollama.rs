use super::super::provider::EmbeddingProvider;
use crate::scheduler::job_name::JobName;
use anyhow::Result;
use async_trait::async_trait;
use ollama_rs::Ollama;
use std::time::Duration;

pub struct OllamaEmbeddingProvider {
  name: String,
  client: Ollama,
}

impl OllamaEmbeddingProvider {
  pub fn new(name: String, client: Ollama) -> Self {
    Self { name, client }
  }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
  fn name(&self) -> String {
    format!("ollama_{}", &self.name)
  }

  fn dimensions(&self) -> usize {
    1024
  }

  fn batch_size(&self) -> usize {
    1
  }

  fn concurrency(&self) -> usize {
    10
  }

  fn interval(&self) -> Duration {
    Duration::from_secs(1)
  }

  fn job_name(&self) -> JobName {
    JobName::GenerateOllamaEmbeddings
  }

  #[tracing::instrument(name = "OllamaEmbeddingProvider::generate", skip_all, fields(count = payloads.len()))]
  async fn generate(&self, payloads: Vec<String>) -> Result<Vec<Vec<f32>>> {
    let mut embeddings = Vec::new();
    for payload in payloads {
      let embedding = self
        .client
        .generate_embeddings(self.name.clone(), payload, None)
        .await?;
      embeddings.push(embedding.embeddings.into_iter().map(|v| v as f32).collect());
    }
    Ok(embeddings)
  }
}
