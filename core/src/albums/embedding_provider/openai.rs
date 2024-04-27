use super::provider::AlbumEmbeddingProvider;
use crate::{
  albums::album_read_model::AlbumReadModel, helpers::key_value_store::KeyValueStore,
  settings::OpenAISettings,
};
use anyhow::Result;
use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
use async_trait::async_trait;
use chrono::Duration;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub struct OpenAIAlbumEmbeddingProvider {
  client: Client<OpenAIConfig>,
  kv: Arc<KeyValueStore>,
}

fn get_document_id(content: String) -> String {
  let mut hasher = Sha256::new();
  hasher.update(content);
  let result = hasher.finalize();
  format!("{:x}", result)
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

  fn get_input(&self, album: &AlbumReadModel) -> (String, String) {
    let mut body = vec![];
    body.push(album.rating.to_string());
    body.push(album.rating_count.to_string());
    if let Some(release_date) = album.release_date {
      body.push(release_date.to_string());
    }
    body.extend(album.artists.clone().into_iter().map(|artist| artist.name));
    body.extend(album.primary_genres.clone());
    body.extend(album.secondary_genres.clone());
    body.extend(album.descriptors.clone());
    body.extend(album.languages.clone());
    body.extend(album.credits.clone().into_iter().map(|c| c.artist.name));
    let body = body.join(", ");
    let id = get_document_id(body.clone());
    (id, body)
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
    let (id, body) = self.get_input(album);
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
