use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use chrono::NaiveDate;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::SetCondition,
  commands::{JsonCommands, JsonGetOptions},
};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct AlbumReadModelArtist {
  pub name: String,
  pub file_name: FileName,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct AlbumReadModelTrack {
  pub name: String,
  pub duration_seconds: Option<u32>,
  pub rating: Option<f32>,
  pub position: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct AlbumReadModel {
  pub name: String,
  pub file_name: FileName,
  pub rating: f32,
  pub rating_count: u32,
  pub artists: Vec<AlbumReadModelArtist>,
  pub primary_genres: Vec<String>,
  pub secondary_genres: Vec<String>,
  pub descriptors: Vec<String>,
  pub tracks: Vec<AlbumReadModelTrack>,
  pub release_date: Option<NaiveDate>,
}

pub struct AlbumReadModelRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl AlbumReadModelRepository {
  pub fn key(&self, file_name: &FileName) -> String {
    format!("album:{}", file_name.to_string())
  }

  pub async fn put(&self, album: AlbumReadModel) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection
      .json_set(
        self.key(&album.file_name),
        "$",
        serde_json::to_string(&album)?,
        SetCondition::default(),
      )
      .await?;
    Ok(())
  }

  pub async fn get(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection
      .json_get(self.key(file_name), JsonGetOptions::default())
      .await?;
    let record = result.map(|r| serde_json::from_str(&r)).transpose()?;

    Ok(record)
  }
}
