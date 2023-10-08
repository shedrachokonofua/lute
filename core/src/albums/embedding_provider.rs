use std::sync::Arc;

use crate::{albums::album_repository::AlbumReadModel, settings::Settings};
use anyhow::Result;
use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
use async_trait::async_trait;

#[async_trait]
pub trait AlbumEmbeddingProvider {
  fn name(&self) -> &str;
  /**
   * Returns a mapping of embedding keys to embeddings for the given album.
   */
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<(&str, Vec<f32>)>>;
}

pub struct OpenAIAlbumEmbeddingProvider {
  client: Client<OpenAIConfig>,
}

impl OpenAIAlbumEmbeddingProvider {
  pub fn new(settings: Arc<Settings>) -> Result<Self> {
    match &settings.openai {
      Some(openai) => Ok(Self {
        client: Client::with_config(OpenAIConfig::default().with_api_key(&openai.api_key)),
      }),
      None => Err(anyhow::anyhow!("OpenAI settings not found")),
    }
  }
}

#[async_trait]
impl AlbumEmbeddingProvider for OpenAIAlbumEmbeddingProvider {
  fn name(&self) -> &str {
    "openai"
  }

  #[tracing::instrument(name = "OpenAIAlbumEmbeddingProvider::generate", skip(self))]
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<(&str, Vec<f32>)>> {
    let request = CreateEmbeddingRequestArgs::default()
      .model("text-embedding-ada-002")
      .input([serde_json::to_string(album)?])
      .build()?;
    let mut response = self.client.embeddings().create(request).await?;
    let mut result = Vec::new();
    result.push((
      "openai-default",
      response
        .data
        .pop()
        .map(|embedding| embedding.embedding)
        .ok_or(anyhow::anyhow!("No embeddings found"))?,
    ));
    Ok(result)
  }
}
