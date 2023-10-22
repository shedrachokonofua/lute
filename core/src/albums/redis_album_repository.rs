use super::album_repository::{
  embedding_to_bytes, AlbumEmbedding, AlbumReadModel, AlbumReadModelArtist, AlbumReadModelCredit,
  AlbumReadModelTrack, AlbumRepository, AlbumSearchQuery, AlbumSearchResult, GenreAggregate,
  ItemAndCount, SearchPagination, SimilarAlbumsQuery,
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
    FtAggregateOptions, FtCreateOptions, FtFieldSchema, FtFieldType, FtFlatVectorFieldAttributes,
    FtIndexDataType, FtReducer, FtSearchOptions, FtVectorDistanceMetric, FtVectorFieldAlgorithm,
    FtVectorType, GenericCommands, JsonCommands, JsonGetOptions, SearchCommands, SetCondition,
    SortOrder,
  },
};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::try_join;
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
  #[serde(default)]
  pub duplicate_of: Option<FileName>,
  #[serde(default)]
  pub is_duplicate: u8,
  #[serde(default)]
  pub duplicates: Vec<FileName>,
  #[serde(default)]
  pub name_tag: String, // redisearch doesn't support exact matching on text fields, so we need to store a tag for exact matching
  #[serde(default)]
  pub cover_image_url: Option<String>,
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
      duplicate_of: self.duplicate_of,
      duplicates: self.duplicates,
      cover_image_url: self.cover_image_url,
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
    let is_duplicate = if self.duplicate_of.is_some() { 1 } else { 0 };

    RedisAlbumReadModel {
      name_tag: self.name.clone(),
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
      duplicate_of: self.duplicate_of,
      duplicates: self.duplicates,
      is_duplicate,
      cover_image_url: self.cover_image_url,
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

impl AlbumSearchQuery {
  pub fn to_ft_search_query(&self) -> String {
    let mut ft_search_query = String::from("");
    if let Some(text) = &self.text {
      ft_search_query.push_str(&format!(
        "({}*) ",
        escape_tag_value(text).replace(r"\ ", " ").trim()
      ));
    }
    if let Some(exact_name) = &self.exact_name {
      ft_search_query.push_str(&get_tag_query("@name_tag", &vec![exact_name]));
    }
    if !self.include_duplicates.is_some_and(|b| b == true) {
      ft_search_query.push_str(&&get_num_range_query("@is_duplicate", Some(0), Some(0)));
    }
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
    ft_search_query.push_str(&get_tag_query("@file_name", &self.include_file_names));
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

impl SimilarAlbumsQuery {
  pub fn to_ft_search_query(&self) -> String {
    format!(
      "({} {})=>[KNN {} @embedding $BLOB as distance]",
      get_tag_query("@embedding_key", &vec![self.embedding_key.clone()]),
      self.filters.to_ft_search_query(),
      self.limit
    )
  }
}

pub struct RedisAlbumRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

const NAMESPACE: &str = "album";
const INDEX_NAME: &str = "album_idx";

impl RedisAlbumRepository {
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
            FtFieldSchema::identifier("$.embeddings..key")
              .as_attribute("embedding_key")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.embeddings..embedding")
              .as_attribute("embedding")
              .field_type(FtFieldType::Vector(Some(FtVectorFieldAlgorithm::Flat(
                FtFlatVectorFieldAttributes::new(
                  FtVectorType::Float32,
                  1536,
                  FtVectorDistanceMetric::Cosine,
                ),
              )))),
            FtFieldSchema::identifier("$.is_duplicate")
              .as_attribute("is_duplicate")
              .field_type(FtFieldType::Numeric),
            FtFieldSchema::identifier("$.name_tag")
              .as_attribute("name_tag")
              .field_type(FtFieldType::Tag),
          ],
        )
        .await?;
    }
    Ok(())
  }

  pub fn key(&self, file_name: &FileName) -> String {
    format!("{}:{}", NAMESPACE, file_name.to_string())
  }

  pub async fn ensure_embeddings_field(&self, file_name: &FileName) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection
      .json_get(
        self.key(file_name),
        JsonGetOptions::default().path("$.embeddings"),
      )
      .await?;
    if result.is_none() || result.is_some_and(|r| r == "[]") {
      connection
        .json_set(
          self.key(file_name),
          "$.embeddings",
          "{}",
          SetCondition::default(),
        )
        .await?;
    }
    Ok(())
  }

  async fn get_aggregated_primary_genres(&self) -> Result<Vec<ItemAndCount>> {
    let connection = self.redis_connection_pool.get().await?;
    let primary_genre_result = connection
      .ft_aggregate(
        INDEX_NAME,
        "@primary_genre_count:[1 +inf]",
        FtAggregateOptions::default()
          .groupby("@primary_genre", FtReducer::count().as_name("count")),
      )
      .await?;

    Ok(
      primary_genre_result
        .results
        .iter()
        .map(ItemAndCount::try_from)
        .filter_map(|r| match r {
          Ok(item) => Some(item),
          Err(e) => {
            info!("Failed to parse primary genre: {}", e);
            None
          }
        })
        .collect(),
    )
  }

  async fn get_aggregated_secondary_genres(&self) -> Result<Vec<ItemAndCount>> {
    let connection = self.redis_connection_pool.get().await?;
    let secondary_genre_result = connection
      .ft_aggregate(
        INDEX_NAME,
        "@secondary_genre_count:[1 +inf]",
        FtAggregateOptions::default()
          .groupby("@secondary_genre", FtReducer::count().as_name("count")),
      )
      .await?;

    Ok(
      secondary_genre_result
        .results
        .iter()
        .map(ItemAndCount::try_from)
        .filter_map(|r| match r {
          Ok(item) => Some(item),
          Err(e) => {
            info!("Failed to parse secondary genre: {}", e);
            None
          }
        })
        .collect(),
    )
  }
}

#[async_trait]
impl AlbumRepository for RedisAlbumRepository {
  async fn put(&self, album: AlbumReadModel) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
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
  async fn search(
    &self,
    query: &AlbumSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<AlbumSearchResult> {
    let limit = pagination.and_then(|p| p.limit).unwrap_or_else(|| 100000);
    let offset = pagination.and_then(|p| p.offset).unwrap_or_else(|| 0);

    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_search(
        INDEX_NAME,
        query.to_ft_search_query(),
        FtSearchOptions::default().limit(offset, limit),
      )
      .await?;

    let albums = result
      .results
      .iter()
      .filter_map(|row| {
        RedisAlbumReadModel::try_from(&row.values)
          .ok()
          .map(|r| r.into())
      })
      .collect();

    Ok(AlbumSearchResult {
      albums,
      total: result.total_results,
    })
  }

  async fn get_aggregated_genres(&self) -> Result<Vec<GenreAggregate>> {
    let (aggregated_primary_genres, aggregated_secondary_genres) = try_join!(
      self.get_aggregated_primary_genres(),
      self.get_aggregated_secondary_genres()
    )?;
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

  #[instrument(skip(self))]
  async fn put_embedding(&self, embedding: &AlbumEmbedding) -> Result<()> {
    self.ensure_embeddings_field(&embedding.file_name).await?;
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        self.key(&embedding.file_name),
        format!("$.embeddings.{}", embedding.key),
        serde_json::to_string(embedding)?,
        SetCondition::default(),
      )
      .await?;
    Ok(())
  }

  async fn get_embeddings(&self, file_name: &FileName) -> Result<Vec<AlbumEmbedding>> {
    let result: Option<String> = self
      .redis_connection_pool
      .get()
      .await?
      .json_get(
        self.key(file_name),
        JsonGetOptions::default().path("$.embeddings.."),
      )
      .await?;
    let embeddings = result
      .map(|r| serde_json::from_str::<Vec<AlbumEmbedding>>(&r))
      .transpose()?
      .unwrap_or_default();
    Ok(embeddings)
  }

  async fn find_many_embeddings(
    &self,
    file_names: Vec<FileName>,
    key: &str,
  ) -> Result<Vec<AlbumEmbedding>> {
    let connection = self.redis_connection_pool.get().await?;
    let keys: Vec<String> = file_names
      .iter()
      .map(|file_name| self.key(file_name))
      .collect();
    let result: Vec<String> = connection
      .json_mget(keys, format!("$.embeddings.{}", key))
      .await?;
    let embeddings = result
      .into_iter()
      .filter_map(|r| {
        serde_json::from_str::<Vec<AlbumEmbedding>>(&r)
          .ok()
          .and_then(|r| r.into_iter().next())
      })
      .collect::<Vec<AlbumEmbedding>>();
    Ok(embeddings)
  }

  async fn delete_embedding(&self, file_name: &FileName, key: &str) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .json_del(self.key(file_name), format!("$.embeddings.{}", key))
      .await?;
    Ok(())
  }

  async fn find_embedding(
    &self,
    file_name: &FileName,
    key: &str,
  ) -> Result<Option<AlbumEmbedding>> {
    let result: Option<String> = self
      .redis_connection_pool
      .get()
      .await?
      .json_get(
        self.key(file_name),
        JsonGetOptions::default().path(format!("$.embeddings.{}", key)),
      )
      .await?;
    let embedding = result
      .map(|r| serde_json::from_str::<AlbumEmbedding>(&r))
      .transpose()?;
    Ok(embedding)
  }

  #[instrument(skip(self))]
  async fn find_similar_albums(
    &self,
    query: &SimilarAlbumsQuery,
  ) -> Result<Vec<(AlbumReadModel, f32)>> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_search(
        INDEX_NAME,
        query.to_ft_search_query(),
        FtSearchOptions::default()
          .params(("BLOB", embedding_to_bytes(&query.embedding)))
          .dialect(2)
          .limit(0, query.limit)
          .sortby("distance", SortOrder::Asc),
      )
      .await?;
    let albums = result
      .results
      .iter()
      .filter_map(|row| {
        let distance = row
          .values
          .get(0)
          .map(|(_, distance)| distance.parse::<f32>().ok())??;
        let redis_album_read_model = row
          .values
          .get(1)
          .and_then(|(_, json)| serde_json::from_str::<RedisAlbumReadModel>(json).ok())?;
        let album_read_model: AlbumReadModel = redis_album_read_model.into();
        Some((album_read_model, distance))
      })
      .collect::<Vec<(AlbumReadModel, f32)>>();
    Ok(albums)
  }

  async fn get_embedding_keys(&self) -> Result<Vec<String>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Vec<String> = connection.ft_tagvals(INDEX_NAME, "embedding_key").await?;
    Ok(result)
  }

  async fn set_duplicate_of(&self, file_name: &FileName, duplicate_of: &FileName) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        self.key(file_name),
        "$.duplicate_of",
        serde_json::to_string(duplicate_of)?,
        SetCondition::default(),
      )
      .await?;
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        self.key(file_name),
        "$.is_duplicate",
        serde_json::to_string(&1)?,
        SetCondition::default(),
      )
      .await?;
    Ok(())
  }

  async fn set_duplicates(&self, file_name: &FileName, duplicates: Vec<FileName>) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        self.key(file_name),
        "$.duplicates",
        serde_json::to_string(&duplicates)?,
        SetCondition::default(),
      )
      .await?;
    Ok(())
  }
}
