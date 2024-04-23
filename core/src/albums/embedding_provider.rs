use std::sync::Arc;

use crate::{albums::album_read_model::AlbumReadModel, settings::Settings};
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

  fn get_input(&self, album: &AlbumReadModel) -> String {
    let mut corpus = vec![];
    corpus.push(album.rating.to_string());
    corpus.push(album.rating_count.to_string());
    if let Some(release_date) = album.release_date {
      corpus.push(release_date.to_string());
    }
    corpus.extend(album.artists.clone().into_iter().map(|artist| artist.name));
    corpus.extend(album.primary_genres.clone());
    corpus.extend(album.secondary_genres.clone());
    corpus.extend(album.descriptors.clone());
    corpus.extend(album.languages.clone());
    corpus.extend(album.credits.clone().into_iter().map(|c| c.artist.name));
    corpus.join(", ")
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
      .model("text-embedding-3-large")
      .input([self.get_input(album)])
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
