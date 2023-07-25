use crate::{
  files::file_metadata::file_name::FileName,
  helpers::redisearch::{does_ft_index_exist, escape_tag_value},
  proto,
};
use anyhow::{anyhow, Error, Result};
use chrono::NaiveDate;
use derive_builder::Builder;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtSearchOptions, GenericCommands,
    JsonCommands, JsonGetOptions, SearchCommands, SetCondition,
  },
};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct AlbumReadModelArtist {
  pub name: String,
  pub file_name: FileName,
}

impl From<AlbumReadModelTrack> for proto::Track {
  fn from(val: AlbumReadModelTrack) -> Self {
    proto::Track {
      name: val.name,
      duration_seconds: val.duration_seconds,
      rating: val.rating,
      position: val.position,
    }
  }
}

impl From<AlbumReadModelArtist> for proto::AlbumArtist {
  fn from(val: AlbumReadModelArtist) -> Self {
    proto::AlbumArtist {
      name: val.name,
      file_name: val.file_name.to_string(),
    }
  }
}

impl From<AlbumReadModel> for proto::Album {
  fn from(val: AlbumReadModel) -> Self {
    proto::Album {
      name: val.name,
      file_name: val.file_name.to_string(),
      rating: val.rating,
      rating_count: val.rating_count,
      artists: val
        .artists
        .into_iter()
        .map(|artist| artist.into())
        .collect(),
      primary_genres: val.primary_genres,
      secondary_genres: val.secondary_genres,
      descriptors: val.descriptors,
      tracks: val.tracks.into_iter().map(|track| track.into()).collect(),
      release_date: val.release_date.map(|date| date.to_string()),
    }
  }
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

impl TryFrom<&Vec<(String, String)>> for AlbumReadModel {
  type Error = Error;

  fn try_from(values: &Vec<(String, String)>) -> Result<Self> {
    let json = values
      .get(0)
      .map(|(_, json)| json)
      .ok_or(anyhow!("invalid AlbumReadModel: missing json"))?;
    let subscription: AlbumReadModel = serde_json::from_str(json)?;
    Ok(subscription)
  }
}

#[derive(Default, Builder, Debug)]
#[builder(setter(into), default)]
pub struct AlbumSearchQuery {
  exclude_file_names: Vec<FileName>,
  include_artists: Vec<String>,
  exclude_artists: Vec<String>,
  include_primary_genres: Vec<String>,
  exclude_primary_genres: Vec<String>,
  include_secondary_genres: Vec<String>,
  exclude_secondary_genres: Vec<String>,
  include_descriptors: Vec<String>,
}

pub struct AlbumReadModelRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

fn get_tag_query<T: ToString>(tag: String, items: &Vec<T>) -> String {
  if !items.is_empty() {
    format!(
      "{}:{{{}}} ",
      tag,
      items
        .iter()
        .map(|item| escape_tag_value(item.to_string().as_str()))
        .collect::<Vec<String>>()
        .join("|")
    )
  } else {
    String::from("")
  }
}

const NAMESPACE: &str = "album";
const INDEX_NAME: &str = "album_idx";

impl AlbumReadModelRepository {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool,
    }
  }

  pub fn key(&self, file_name: &FileName) -> String {
    format!("{}:{}", NAMESPACE, file_name.to_string())
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

  pub async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection
      .json_get(self.key(file_name), JsonGetOptions::default())
      .await?;
    let record = result.map(|r| serde_json::from_str(&r)).transpose()?;

    Ok(record)
  }

  pub async fn get(&self, file_name: &FileName) -> Result<AlbumReadModel> {
    let record = self.find(file_name).await?;
    match record {
      Some(record) => Ok(record),
      None => anyhow::bail!("Album does not exist"),
    }
  }

  pub async fn exists(&self, file_name: &FileName) -> Result<bool> {
    let connection = self.redis_connection_pool.get().await?;
    let result: usize = connection.exists(self.key(file_name)).await?;
    Ok(result == 1)
  }

  pub async fn get_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>> {
    let connection = self.redis_connection_pool.get().await?;
    let keys: Vec<String> = file_names
      .iter()
      .map(|file_name| self.key(file_name))
      .collect();
    let result: Vec<String> = connection.json_mget(keys, "$").await?;
    let records = result
      .into_iter()
      .map(|r| -> Result<Vec<AlbumReadModel>> {
        serde_json::from_str(&r).map_err(|e| anyhow::anyhow!(e))
      })
      .collect::<Result<Vec<Vec<AlbumReadModel>>>>()?;
    let data = records
      .into_iter()
      .flat_map(|r| r.into_iter())
      .collect::<Vec<AlbumReadModel>>();

    Ok(data)
  }

  pub async fn search(
    &self,
    query: &AlbumSearchQuery,
    offset: Option<usize>,
    limit: Option<usize>,
  ) -> Result<Vec<AlbumReadModel>> {
    let mut redis_query = String::from("");
    redis_query.push_str(&get_tag_query(
      "-@file_name".to_string(),
      &query.exclude_file_names,
    ));
    redis_query.push_str(&get_tag_query(
      "@artist_file_name".to_string(),
      &query.include_artists,
    ));
    redis_query.push_str(&get_tag_query(
      "-@artist_file_name".to_string(),
      &query.exclude_artists,
    ));
    redis_query.push_str(&get_tag_query(
      "@primary_genre".to_string(),
      &query.include_primary_genres,
    ));
    redis_query.push_str(&get_tag_query(
      "-@primary_genre".to_string(),
      &query.exclude_primary_genres,
    ));
    redis_query.push_str(&get_tag_query(
      "@secondary_genre".to_string(),
      &query.include_secondary_genres,
    ));
    redis_query.push_str(&get_tag_query(
      "-@secondary_genre".to_string(),
      &query.exclude_secondary_genres,
    ));
    redis_query.push_str(&get_tag_query(
      "@descriptor".to_string(),
      &query.include_descriptors,
    ));
    let redis_query = redis_query.trim().to_string();
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_search(
        INDEX_NAME,
        redis_query,
        FtSearchOptions::default().limit(offset.unwrap_or(0), limit.unwrap_or(100)),
      )
      .await?;
    let albums = result
      .results
      .iter()
      .map(|row| AlbumReadModel::try_from(&row.values))
      .filter_map(|r| match r {
        Ok(album) => Some(album),
        Err(e) => {
          info!("Failed to parse album: {}", e);
          None
        }
      })
      .collect::<Vec<_>>();
    Ok(albums)
  }

  pub async fn setup_index(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    if !does_ft_index_exist(&connection, INDEX_NAME).await {
      info!("Creating index {}", INDEX_NAME);
      connection
        .ft_create(
          INDEX_NAME,
          FtCreateOptions::default()
            .on(FtIndexDataType::Json)
            .prefix(format!("{}:", NAMESPACE)),
          [
            FtFieldSchema::identifier("$.name")
              .as_attribute("name")
              .field_type(FtFieldType::Text),
            FtFieldSchema::identifier("$.file_name")
              .as_attribute("file_name")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.artists[*].name")
              .as_attribute("artist_name")
              .field_type(FtFieldType::Text),
            FtFieldSchema::identifier("$.artists[*].file_name")
              .as_attribute("artist_file_name")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.primary_genres.*")
              .as_attribute("primary_genre")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.secondary_genres.*")
              .as_attribute("secondary_genre")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.descriptors.*")
              .as_attribute("descriptor")
              .field_type(FtFieldType::Tag),
          ],
        )
        .await?;
    }
    Ok(())
  }
}
