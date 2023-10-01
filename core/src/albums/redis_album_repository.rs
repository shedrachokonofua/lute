use super::album_repository::{
  AlbumReadModel, AlbumReadModelArtist, AlbumReadModelCredit, AlbumReadModelTrack, AlbumRepository,
  AlbumSearchQuery, GenreAggregate, ItemAndCount,
};
use crate::{
  files::file_metadata::file_name::FileName,
  helpers::redisearch::{does_ft_index_exist, escape_tag_value},
};
use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtAggregateOptions, FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtReducer,
    FtSearchOptions, GenericCommands, JsonCommands, JsonGetOptions, SearchCommands, SetCondition,
  },
};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, instrument};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct RedisAlbumReadModel {
  pub name: String,
  pub file_name: FileName,
  pub rating: f32,
  pub rating_count: u32,
  pub artists: Vec<AlbumReadModelArtist>,
  pub artist_count: u32,
  pub primary_genres: Vec<String>,
  pub primary_genre_count: u32,
  pub secondary_genres: Vec<String>,
  pub secondary_genre_count: u32,
  pub descriptors: Vec<String>,
  pub descriptor_count: u32,
  pub tracks: Vec<AlbumReadModelTrack>,
  pub release_date: Option<NaiveDate>,
  pub release_year: Option<u32>,
  #[serde(default)]
  pub languages: Vec<String>,
  #[serde(default)]
  pub language_count: u32,
  #[serde(default)]
  pub credits: Vec<AlbumReadModelCredit>,
  #[serde(default)]
  pub credit_tags: Vec<String>,
  #[serde(default)]
  pub credit_tag_count: u32,
}

impl Into<AlbumReadModel> for RedisAlbumReadModel {
  fn into(self) -> AlbumReadModel {
    AlbumReadModel {
      name: self.name,
      file_name: self.file_name,
      rating: self.rating,
      rating_count: self.rating_count,
      artists: self.artists,
      primary_genres: self.primary_genres,
      secondary_genres: self.secondary_genres,
      descriptors: self.descriptors,
      tracks: self.tracks,
      release_date: self.release_date,
      languages: self.languages,
      credits: self.credits,
    }
  }
}

impl Into<RedisAlbumReadModel> for AlbumReadModel {
  fn into(self) -> RedisAlbumReadModel {
    let artist_count = self.artists.len() as u32;
    let primary_genre_count = self.primary_genres.len() as u32;
    let secondary_genre_count = self.secondary_genres.len() as u32;
    let descriptor_count = self.descriptors.len() as u32;
    let language_count = self.languages.len() as u32;
    let credit_tags = self.credit_tags();
    let credit_tag_count = credit_tags.len() as u32;
    let release_year = self.release_date.map(|d| d.year() as u32);

    RedisAlbumReadModel {
      name: self.name,
      file_name: self.file_name,
      rating: self.rating,
      rating_count: self.rating_count,
      artists: self.artists,
      artist_count,
      primary_genres: self.primary_genres,
      primary_genre_count,
      secondary_genres: self.secondary_genres,
      secondary_genre_count,
      descriptors: self.descriptors,
      descriptor_count,
      tracks: self.tracks,
      release_date: self.release_date,
      release_year,
      languages: self.languages,
      language_count,
      credits: self.credits,
      credit_tags,
      credit_tag_count,
    }
  }
}

impl TryFrom<&Vec<(String, String)>> for RedisAlbumReadModel {
  type Error = Error;

  fn try_from(values: &Vec<(String, String)>) -> Result<Self> {
    let json = values
      .get(0)
      .map(|(_, json)| json)
      .ok_or(anyhow!("invalid AlbumReadModel: missing json"))?;
    let album: RedisAlbumReadModel = serde_json::from_str(json)?;
    Ok(album)
  }
}

impl TryFrom<&Vec<(String, String)>> for ItemAndCount {
  type Error = Error;

  fn try_from(values: &Vec<(String, String)>) -> Result<Self> {
    let name = values
      .get(0)
      .map(|(_, name)| name)
      .ok_or(anyhow!("invalid ItemAndCount: missing name"))?;
    let count = values
      .get(1)
      .map(|(_, count)| count)
      .ok_or(anyhow!("invalid ItemAndCount: missing count"))?;
    Ok(ItemAndCount {
      name: name.to_string(),
      count: count.parse()?,
    })
  }
}

impl AlbumSearchQuery {
  pub fn to_ft_search_query(&self) -> String {
    let mut ft_search_query = String::from("");
    ft_search_query.push_str(&get_min_num_query(
      "@primary_genre_count",
      self.min_primary_genre_count,
    ));
    ft_search_query.push_str(&get_min_num_query(
      "@secondary_genre_count",
      self.min_secondary_genre_count,
    ));
    ft_search_query.push_str(&get_min_num_query(
      "@descriptor_count",
      self.min_descriptor_count,
    ));
    ft_search_query.push_str(&get_num_range_query(
      "@release_year",
      self.min_release_year,
      self.max_release_year,
    ));
    ft_search_query.push_str(&get_tag_query("@artist_file_name", &self.include_artists));
    ft_search_query.push_str(&get_tag_query(
      "@primary_genre",
      &self.include_primary_genres,
    ));
    ft_search_query.push_str(&get_tag_query(
      "@secondary_genre",
      &self.include_secondary_genres,
    ));
    ft_search_query.push_str(&get_tag_query("@language", &self.include_languages));
    ft_search_query.push_str(&get_tag_query("@descriptor", &self.include_descriptors));
    ft_search_query.push_str(&get_tag_query("-@artist_file_name", &self.exclude_artists));
    ft_search_query.push_str(&get_tag_query("-@file_name", &self.exclude_file_names));
    ft_search_query.push_str(&get_tag_query(
      "-@primary_genre",
      &self.exclude_primary_genres,
    ));
    ft_search_query.push_str(&get_tag_query(
      "-@secondary_genre",
      &self.exclude_secondary_genres,
    ));
    ft_search_query.push_str(&get_tag_query("-@language", &self.exclude_languages));
    return ft_search_query.trim().to_string();
  }
}

pub struct RedisAlbumReadModelRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

fn get_tag_query<T: ToString>(tag: &str, items: &Vec<T>) -> String {
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

fn get_min_num_query(tag: &str, min: Option<usize>) -> String {
  if let Some(min) = min {
    format!("{}:[{}, +inf] ", tag, min)
  } else {
    String::from("")
  }
}

fn get_num_range_query(tag: &str, min: Option<u32>, max: Option<u32>) -> String {
  match (min, max) {
    (Some(min), Some(max)) => format!("{}:[{}, {}] ", tag, min, max),
    (Some(min), None) => format!("{}:[{}, +inf] ", tag, min),
    (None, Some(max)) => format!("{}:[-inf, {}] ", tag, max),
    (None, None) => String::from(""),
  }
}

const NAMESPACE: &str = "album";
const INDEX_NAME: &str = "album_idx";

impl RedisAlbumReadModelRepository {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool,
    }
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
            FtFieldSchema::identifier("$.rating")
              .as_attribute("rating")
              .field_type(FtFieldType::Numeric),
            FtFieldSchema::identifier("$.rating_count")
              .as_attribute("rating_count")
              .field_type(FtFieldType::Numeric),
            FtFieldSchema::identifier("$.primary_genres.*")
              .as_attribute("primary_genre")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.primary_genre_count")
              .as_attribute("primary_genre_count")
              .field_type(FtFieldType::Numeric),
            FtFieldSchema::identifier("$.secondary_genres.*")
              .as_attribute("secondary_genre")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.secondary_genre_count")
              .as_attribute("secondary_genre_count")
              .field_type(FtFieldType::Numeric),
            FtFieldSchema::identifier("$.descriptors.*")
              .as_attribute("descriptor")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.descriptor_count")
              .as_attribute("descriptor_count")
              .field_type(FtFieldType::Numeric),
            FtFieldSchema::identifier("$.release_year")
              .as_attribute("release_year")
              .field_type(FtFieldType::Numeric),
            FtFieldSchema::identifier("$.languages.*")
              .as_attribute("language")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.language_count")
              .as_attribute("language_count")
              .field_type(FtFieldType::Numeric),
          ],
        )
        .await?;
    }
    Ok(())
  }

  pub fn key(&self, file_name: &FileName) -> String {
    format!("{}:{}", NAMESPACE, file_name.to_string())
  }
}

#[async_trait]
impl AlbumRepository for RedisAlbumReadModelRepository {
  async fn put(&self, album: AlbumReadModel) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection
      .json_set(
        self.key(&album.file_name),
        "$",
        serde_json::to_string::<RedisAlbumReadModel>(&album.into())?,
        SetCondition::default(),
      )
      .await?;
    Ok(())
  }

  async fn delete(&self, file_name: &FileName) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection.del(self.key(file_name)).await?;
    Ok(())
  }

  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection
      .json_get(self.key(file_name), JsonGetOptions::default())
      .await?;
    let record = result
      .map(|r| serde_json::from_str::<RedisAlbumReadModel>(&r))
      .transpose()?
      .map(|r| r.into());

    Ok(record)
  }

  async fn exists(&self, file_name: &FileName) -> Result<bool> {
    let connection = self.redis_connection_pool.get().await?;
    let result: usize = connection.exists(self.key(file_name)).await?;
    Ok(result == 1)
  }

  async fn get_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>> {
    let connection = self.redis_connection_pool.get().await?;
    let keys: Vec<String> = file_names
      .iter()
      .map(|file_name| self.key(file_name))
      .collect();
    let result: Vec<String> = connection.json_mget(keys, "$").await?;
    let records = result
      .into_iter()
      .map(|r| -> Result<Vec<RedisAlbumReadModel>> {
        serde_json::from_str(&r).map_err(|e| anyhow::anyhow!(e))
      })
      .collect::<Result<Vec<Vec<RedisAlbumReadModel>>>>()?;
    let data = records
      .into_iter()
      .flat_map(|r| r.into_iter())
      .map(|r| r.into())
      .collect::<Vec<AlbumReadModel>>();

    Ok(data)
  }

  #[instrument(skip(self))]
  async fn search(&self, query: &AlbumSearchQuery) -> Result<Vec<AlbumReadModel>> {
    let page_size: usize = 100_000;
    let mut albums: Vec<AlbumReadModel> = Vec::new();
    let mut offset = 0;

    loop {
      let connection = self.redis_connection_pool.get().await?;
      let result = connection
        .ft_search(
          INDEX_NAME,
          query.to_ft_search_query(),
          FtSearchOptions::default().limit(offset, page_size),
        )
        .await?;

      albums.extend(result.results.iter().filter_map(|row| {
        RedisAlbumReadModel::try_from(&row.values)
          .ok()
          .map(|r| r.into())
      }));

      if result.results.len() < page_size {
        break;
      }
      offset += page_size;
    }

    Ok(albums)
  }

  async fn get_aggregated_genres(&self) -> Result<Vec<GenreAggregate>> {
    let connection = self.redis_connection_pool.get().await?;
    let primary_genre_result = connection
      .ft_aggregate(
        INDEX_NAME,
        "@primary_genre_count:[1 +inf]",
        FtAggregateOptions::default()
          .groupby("@primary_genre", FtReducer::count().as_name("count")),
      )
      .await?;
    let secondary_genre_result = connection
      .ft_aggregate(
        INDEX_NAME,
        "@secondary_genre_count:[1 +inf]",
        FtAggregateOptions::default()
          .groupby("@secondary_genre", FtReducer::count().as_name("count")),
      )
      .await?;
    let aggregated_primary_genres = primary_genre_result
      .results
      .iter()
      .map(ItemAndCount::try_from)
      .filter_map(|r| match r {
        Ok(item) => Some(item),
        Err(e) => {
          info!("Failed to parse genre: {}", e);
          None
        }
      })
      .collect::<Vec<ItemAndCount>>();
    let aggregated_secondary_genres = secondary_genre_result
      .results
      .iter()
      .map(ItemAndCount::try_from)
      .filter_map(|r| match r {
        Ok(item) => Some(item),
        Err(e) => {
          info!("Failed to parse genre: {}", e);
          None
        }
      })
      .collect::<Vec<ItemAndCount>>();
    let mut genres = HashMap::new();
    for item in aggregated_primary_genres {
      genres.insert(
        item.name.clone(),
        GenreAggregate {
          name: item.name,
          primary_genre_count: item.count,
          secondary_genre_count: 0,
        },
      );
    }
    for item in aggregated_secondary_genres {
      if let Some(genre) = genres.get_mut(&item.name) {
        genre.secondary_genre_count = item.count;
      } else {
        genres.insert(
          item.name.clone(),
          GenreAggregate {
            name: item.name,
            primary_genre_count: 0,
            secondary_genre_count: item.count,
          },
        );
      }
    }
    let mut genres = genres.into_values().collect::<Vec<GenreAggregate>>();
    genres.sort_by(|a, b| {
      let a_total = a.primary_genre_count + a.secondary_genre_count;
      let b_total = b.primary_genre_count + b.secondary_genre_count;
      b_total.cmp(&a_total)
    });
    Ok(genres)
  }

  async fn get_aggregated_descriptors(&self) -> Result<Vec<ItemAndCount>> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_aggregate(
        INDEX_NAME,
        "@descriptor_count:[1 +inf]",
        FtAggregateOptions::default().groupby("@descriptor", FtReducer::count().as_name("count")),
      )
      .await?;
    let mut aggregated_descriptors = result
      .results
      .iter()
      .map(ItemAndCount::try_from)
      .filter_map(|r| match r {
        Ok(item) => Some(item),
        Err(e) => {
          info!("Failed to parse descriptor: {}", e);
          None
        }
      })
      .collect::<Vec<ItemAndCount>>();
    aggregated_descriptors.sort_by(|a, b| b.count.cmp(&a.count));
    Ok(aggregated_descriptors)
  }

  async fn get_aggregated_languages(&self) -> Result<Vec<ItemAndCount>> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_aggregate(
        INDEX_NAME,
        "@language_count:[1 +inf]",
        FtAggregateOptions::default().groupby("@language", FtReducer::count().as_name("count")),
      )
      .await?;
    let mut aggregated_languages = result
      .results
      .iter()
      .map(ItemAndCount::try_from)
      .filter_map(|r| match r {
        Ok(item) => Some(item),
        Err(e) => {
          info!("Failed to parse language: {}", e);
          None
        }
      })
      .collect::<Vec<ItemAndCount>>();
    aggregated_languages.sort_by(|a, b| b.count.cmp(&a.count));
    Ok(aggregated_languages)
  }
}
