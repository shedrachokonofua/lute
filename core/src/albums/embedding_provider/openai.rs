use super::{helpers::get_embedding_api_input, provider::AlbumEmbeddingProvider};
use crate::{
  albums::album_read_model::AlbumReadModel, helpers::key_value_store::KeyValueStore,
  settings::OpenAISettings,
};
use anyhow::Result;
use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
use async_trait::async_trait;
use chrono::Duration;
use std::sync::Arc;

pub struct OpenAIAlbumEmbeddingProvider {
  client: Client<OpenAIConfig>,
  kv: Arc<KeyValueStore>,
}

fn document_kv_key(id: &str) -> String {
  format!("openai:doc:{}", id)
}

impl OpenAIAlbumEmbeddingProvider {
  pub fn new(settings: &OpenAISettings, kv: Arc<KeyValueStore>) -> Self {
    Self {
      client: Client::with_config(OpenAIConfig::default().with_api_key(&settings.api_key)),
      kv,
    }
  }
}

#[async_trait]
impl AlbumEmbeddingProvider for OpenAIAlbumEmbeddingProvider {
  fn name(&self) -> &str {
    "openai-default"
  }

  fn dimensions(&self) -> usize {
    3072
  }

  fn concurrency(&self) -> usize {
    25
  }

  #[tracing::instrument(name = "OpenAIAlbumEmbeddingProvider::generate", skip(self))]
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<f32>> {
    let (id, body) = get_embedding_api_input(album);
    if let Some(embedding) = self.kv.get::<Vec<f32>>(&document_kv_key(&id)).await? {
      return Ok(embedding);
    }
    let request = CreateEmbeddingRequestArgs::default()
      .model("text-embedding-3-large")
      .input([body])
      .build()?;
    let mut response = self.client.embeddings().create(request).await?;
    let embedding = response
      .data
      .pop()
      .map(|embedding| embedding.embedding)
      .ok_or(anyhow::anyhow!("No embeddings found"))?;
    self
      .kv
      .set(&document_kv_key(&id), &embedding, Duration::try_weeks(8))
      .await?;
    Ok(embedding)
  }
}
