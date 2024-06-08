use super::provider::AlbumEmbeddingProvider;
use crate::albums::album_read_model::AlbumReadModel;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Datelike;
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct OneHotAlbumEmbeddingProvider;

impl Default for OneHotAlbumEmbeddingProvider {
  fn default() -> Self {
    Self::new()
  }
}

impl OneHotAlbumEmbeddingProvider {
  pub fn new() -> Self {
    Self
  }
}

const DIMENSIONS: usize = 512;

fn to_index(tag: String) -> usize {
  let mut hasher = DefaultHasher::new();
  tag.hash(&mut hasher);
  hasher.finish() as usize % DIMENSIONS
}

fn one_hot_encode(album: &AlbumReadModel) -> Vec<f32> {
  let mut embedding = vec![0.0; DIMENSIONS];
  let mut tags = [
    album.artists.iter().map(|a| a.name.clone()).collect(),
    album
      .credits
      .iter()
      .flat_map(|c| {
        c.roles
          .iter()
          .map(|r| format!("{}:{}", c.artist.name, r))
          .collect::<Vec<String>>()
      })
      .collect(),
    album.primary_genres.clone(),
    album.secondary_genres.clone(),
    album.descriptors.clone(),
    album.languages.clone(),
    vec![
      album.name.clone(),
      album.rating.round().to_string(),
      album.rating_count.next_multiple_of(1000).to_string(),
    ],
  ]
  .concat();
  if let Some(release_date) = album.release_date {
    tags.push(release_date.year().to_string());
  }
  for tag in tags {
    let index = to_index(tag);
    embedding[index] = 1.0;
  }
  embedding
}

#[async_trait]
impl AlbumEmbeddingProvider for OneHotAlbumEmbeddingProvider {
  fn name(&self) -> &str {
    "one-hot-default"
  }

  fn dimensions(&self) -> usize {
    DIMENSIONS
  }

  fn concurrency(&self) -> usize {
    100
  }

  #[tracing::instrument(name = "OneHotAlbumEmbeddingProvider::generate", skip(self))]
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<f32>> {
    Ok(one_hot_encode(album))
  }
}
