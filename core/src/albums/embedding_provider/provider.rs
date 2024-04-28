use crate::albums::album_read_model::AlbumReadModel;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait AlbumEmbeddingProvider {
  fn name(&self) -> &str;
  fn dimensions(&self) -> usize;
  fn concurrency(&self) -> usize;
  /**
   * Returns a mapping of embedding keys to embeddings for the given album.
   */
  async fn generate(&self, album: &AlbumReadModel) -> Result<Vec<f32>>;
}
