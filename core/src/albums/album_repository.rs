use super::album_read_model::AlbumReadModel;
use crate::files::file_metadata::file_name::FileName;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::instrument;

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
  async fn set_duplicates(&self, file_name: &FileName, duplicates: Vec<FileName>) -> Result<()>;
  async fn set_duplicate_of(&self, file_name: &FileName, duplicate_of: &FileName) -> Result<()>;
  async fn delete(&self, file_name: &FileName) -> Result<()>;
  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>>;
  async fn find_artist_albums(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<Vec<AlbumReadModel>>;
  async fn find_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>>;
  async fn get_aggregated_genres(&self, limit: Option<u32>) -> Result<Vec<GenreAggregate>>;
  async fn get_aggregated_descriptors(&self, limit: Option<u32>) -> Result<Vec<ItemAndCount>>;
  async fn get_aggregated_languages(&self, limit: Option<u32>) -> Result<Vec<ItemAndCount>>;
  async fn get_aggregated_years(&self, limit: Option<u32>) -> Result<Vec<ItemAndCount>>;
  async fn get_album_count(&self) -> Result<u32>;
  async fn get_artist_count(&self) -> Result<u32>;
  async fn get_genre_count(&self) -> Result<u32>;
  async fn get_descriptor_count(&self) -> Result<u32>;
  async fn get_language_count(&self) -> Result<u32>;
  async fn get_duplicate_count(&self) -> Result<u32>;

  #[instrument(skip(self))]
  async fn get(&self, file_name: &FileName) -> Result<AlbumReadModel> {
    let record = self.find(file_name).await?;
    match record {
      Some(record) => Ok(record),
      None => anyhow::bail!("Album does not exist"),
    }
  }

  #[instrument(skip_all, fields(count = file_names.len()))]
  async fn get_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>> {
    let albums = self.find_many(file_names.clone()).await?;
    let album_map = albums
      .iter()
      .map(|album| (album.file_name.clone(), album))
      .collect::<HashMap<FileName, &AlbumReadModel>>();
    let missing_file_names = file_names
      .into_iter()
      .filter(|file_name| !album_map.contains_key(file_name))
      .collect::<Vec<FileName>>();
    if missing_file_names.len() > 0 {
      Err(anyhow!(
        "Albums not found: {}",
        missing_file_names
          .iter()
          .map(|file_name| file_name.to_string())
          .collect::<Vec<String>>()
          .join(", ")
      ))
    } else {
      Ok(albums)
    }
  }
}
