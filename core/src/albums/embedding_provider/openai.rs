use super::provider::AlbumEmbeddingProvider;
use crate::{albums::album_read_model::AlbumReadModel, settings::OpenAISettings};
use anyhow::Result;
use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
use async_trait::async_trait;

pub struct OpenAIAlbumEmbeddingProvider {
  client: Client<OpenAIConfig>,
}

impl OpenAIAlbumEmbeddingProvider {
  pub fn new(settings: &OpenAISettings) -> Self {
    Self {
      client: Client::with_config(OpenAIConfig::default().with_api_key(&settings.api_key)),
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
    "openai-default"
  }

  fn dimensions(&self) -> usize {
    3072
  }

  #[tracing::instrument(name = "OpenAIAlbumEmbeddingProvider::generate", skip(self))]
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<f32>> {
    let request = CreateEmbeddingRequestArgs::default()
      .model("text-embedding-3-large")
      .input([self.get_input(album)])
      .build()?;
    let mut response = self.client.embeddings().create(request).await?;
    response
      .data
      .pop()
      .map(|embedding| embedding.embedding)
      .ok_or(anyhow::anyhow!("No embeddings found"))
  }
}
