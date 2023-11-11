use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use async_trait::async_trait;

use super::album_read_model::AlbumReadModel;

pub struct GenreAggregate {
  pub name: String,
  pub primary_genre_count: u32,
  pub secondary_genre_count: u32,
}

pub struct ItemAndCount {
  pub name: String,
  pub count: u32,
}

#[async_trait]
pub trait AlbumRepository {
  async fn put(&self, album: AlbumReadModel) -> Result<()>;
  async fn delete(&self, file_name: &FileName) -> Result<()>;
  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>>;
  async fn find_artist_albums(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<Vec<AlbumReadModel>>;
  async fn get_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>>;
  async fn get_aggregated_genres(&self) -> Result<Vec<GenreAggregate>>;
  async fn get_aggregated_descriptors(&self) -> Result<Vec<ItemAndCount>>;
  async fn get_aggregated_languages(&self) -> Result<Vec<ItemAndCount>>;
  async fn set_duplicates(&self, file_name: &FileName, duplicates: Vec<FileName>) -> Result<()>;
  async fn set_duplicate_of(&self, file_name: &FileName, duplicate_of: &FileName) -> Result<()>;

  async fn get(&self, file_name: &FileName) -> Result<AlbumReadModel> {
    let record = self.find(file_name).await?;
    match record {
      Some(record) => Ok(record),
      None => anyhow::bail!("Album does not exist"),
    }
  }
}
