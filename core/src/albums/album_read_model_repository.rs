use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use chrono::NaiveDate;
use r2d2::Pool;
use redis::{Client, JsonCommands};
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRedisValue, Clone, Default)]
pub struct AlbumReadModelArtist {
  pub name: String,
  pub file_name: FileName,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRedisValue, Clone, Default)]
pub struct AlbumReadModelTrack {
  pub name: String,
  pub duration_seconds: Option<u32>,
  pub rating: Option<f32>,
  pub position: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, FromRedisValue, Clone, Default, ToRedisArgs)]
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
  pub redis_connection_pool: Arc<Pool<Client>>,
}

impl AlbumReadModelRepository {
  pub fn key(&self, file_name: &FileName) -> String {
    format!("album:{}", file_name.to_string())
  }

  pub fn put(&self, album: AlbumReadModel) -> Result<()> {
    let mut connection = self.redis_connection_pool.get().unwrap();
    connection.json_set(self.key(&album.file_name), "$", &album)?;
    Ok(())
  }

  pub fn get(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    let mut connection = self.redis_connection_pool.get().unwrap();
    let result = connection.json_get(self.key(file_name), "$")?;

    Ok(result)
  }
}
