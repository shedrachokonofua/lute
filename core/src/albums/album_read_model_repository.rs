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
    FtAggregateOptions, FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtReducer,
    FtSearchOptions, GenericCommands, JsonCommands, JsonGetOptions, SearchCommands, SetCondition,
  },
};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, instrument};

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

pub struct GenreAggregate {
  name: String,
  primary_genre_count: u32,
  secondary_genre_count: u32,
}

pub struct ItemAndCount {
  name: String,
  count: u32,
}

impl From<&GenreAggregate> for proto::GenreAggregate {
  fn from(val: &GenreAggregate) -> Self {
    proto::GenreAggregate {
      name: val.name.clone(),
      primary_genre_count: val.primary_genre_count,
      secondary_genre_count: val.secondary_genre_count,
    }
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

impl From<&ItemAndCount> for proto::DescriptorAggregate {
  fn from(val: &ItemAndCount) -> Self {
    proto::DescriptorAggregate {
      name: val.name.clone(),
      count: val.count,
    }
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
  min_primary_genre_count: Option<usize>,
  min_secondary_genre_count: Option<usize>,
  min_descriptor_count: Option<usize>,
  min_release_year: Option<u32>,
  max_release_year: Option<u32>,
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
    return ft_search_query.trim().to_string();
  }
}

pub struct AlbumReadModelRepository {
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

  #[instrument(skip(self))]
  pub async fn search(&self, query: &AlbumSearchQuery) -> Result<Vec<AlbumReadModel>> {
    let page_size: usize = 10_000;
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

      albums.extend(
        result
          .results
          .iter()
          .filter_map(|row| AlbumReadModel::try_from(&row.values).ok()),
      );

      if result.results.len() < page_size {
        break;
      }
      offset += page_size;
    }

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
          ],
        )
        .await?;
    }
    Ok(())
  }

  pub async fn get_aggregated_genres(&self) -> Result<Vec<GenreAggregate>> {
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

  pub async fn get_aggregated_descriptors(&self) -> Result<Vec<ItemAndCount>> {
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
}
