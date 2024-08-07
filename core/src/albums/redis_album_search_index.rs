use super::{
  album_read_model::{
    AlbumReadModel, AlbumReadModelArtist, AlbumReadModelBuilder, AlbumReadModelCredit,
    AlbumReadModelTrack,
  },
  album_repository::ItemAndCount,
  album_search_index::{
    AlbumEmbeddingSimilarirtySearchQuery, AlbumSearchIndex, AlbumSearchQuery, AlbumSearchResult,
  },
};
use crate::{
  embedding_provider::embedding_provider_interactor::EmbeddingProviderInteractor,
  files::file_metadata::file_name::FileName,
  helpers::{
    embedding::{embedding_to_bytes, EmbeddingDocument},
    redisearch::{
      escape_search_query_text, get_min_num_query, get_num_range_query, get_tag_query,
      SearchIndexVersionManager, SearchPagination,
    },
  },
};
use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use futures::future::join_all;
use futures::{stream, StreamExt, TryStreamExt};
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtCreateOptions, FtFieldSchema, FtFieldType, FtFlatVectorFieldAttributes, FtIndexDataType,
    FtSearchOptions, FtSearchReturnAttribute, FtVectorDistanceMetric, FtVectorFieldAlgorithm,
    FtVectorType, GenericCommands, JsonCommands, JsonGetOptions, SearchCommands, SetCondition,
    SortOrder,
  },
};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{instrument, warn};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct RedisAlbumReadModelArtist {
  pub name: String,
  pub ascii_name: String,
  pub file_name: FileName,
}

impl From<RedisAlbumReadModelArtist> for AlbumReadModelArtist {
  fn from(val: RedisAlbumReadModelArtist) -> Self {
    AlbumReadModelArtist {
      name: val.name,
      file_name: val.file_name,
    }
  }
}

impl From<AlbumReadModelArtist> for RedisAlbumReadModelArtist {
  fn from(val: AlbumReadModelArtist) -> Self {
    RedisAlbumReadModelArtist {
      name: val.name.clone(),
      ascii_name: val.ascii_name(),
      file_name: val.file_name,
    }
  }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]

pub struct RedisAlbumReadModelCredit {
  pub artist: RedisAlbumReadModelArtist,
  pub roles: Vec<String>,
}

impl From<RedisAlbumReadModelCredit> for AlbumReadModelCredit {
  fn from(val: RedisAlbumReadModelCredit) -> Self {
    AlbumReadModelCredit {
      artist: val.artist.into(),
      roles: val.roles,
    }
  }
}

impl From<AlbumReadModelCredit> for RedisAlbumReadModelCredit {
  fn from(val: AlbumReadModelCredit) -> Self {
    RedisAlbumReadModelCredit {
      artist: val.artist.into(),
      roles: val.roles,
    }
  }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct RedisAlbumReadModel {
  pub name: String,
  pub ascii_name: String,
  pub file_name: FileName,
  pub rating: f32,
  pub rating_count: u32,
  pub artists: Vec<RedisAlbumReadModelArtist>,
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
  pub credits: Vec<RedisAlbumReadModelCredit>,
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
  #[serde(default)]
  pub spotify_id: Option<String>,
}

impl From<RedisAlbumReadModel> for AlbumReadModel {
  fn from(val: RedisAlbumReadModel) -> Self {
    AlbumReadModel {
      name: val.name,
      file_name: val.file_name,
      rating: val.rating,
      rating_count: val.rating_count,
      artists: val.artists.into_iter().map(|a| a.into()).collect(),
      primary_genres: val.primary_genres,
      secondary_genres: val.secondary_genres,
      descriptors: val.descriptors,
      tracks: val.tracks,
      release_date: val.release_date,
      languages: val.languages,
      credits: val.credits.into_iter().map(|c| c.into()).collect(),
      duplicate_of: val.duplicate_of,
      duplicates: val.duplicates,
      cover_image_url: val.cover_image_url,
      spotify_id: val.spotify_id,
    }
  }
}

impl From<AlbumReadModel> for RedisAlbumReadModel {
  fn from(val: AlbumReadModel) -> Self {
    let artist_count = val.artists.len() as u32;
    let primary_genre_count = val.primary_genres.len() as u32;
    let secondary_genre_count = val.secondary_genres.len() as u32;
    let descriptor_count = val.descriptors.len() as u32;
    let language_count = val.languages.len() as u32;
    let credit_tags = val.credit_tags();
    let credit_tag_count = credit_tags.len() as u32;
    let release_year = val.release_date.map(|d| d.year() as u32);
    let is_duplicate = if val.duplicate_of.is_some() { 1 } else { 0 };

    RedisAlbumReadModel {
      name_tag: val.name.clone(),
      name: val.name.clone(),
      ascii_name: val.ascii_name(),
      file_name: val.file_name,
      rating: val.rating,
      rating_count: val.rating_count,
      artists: val.artists.into_iter().map(|a| a.into()).collect(),
      artist_count,
      primary_genres: val.primary_genres,
      primary_genre_count,
      secondary_genres: val.secondary_genres,
      secondary_genre_count,
      descriptors: val.descriptors,
      descriptor_count,
      tracks: val.tracks,
      release_date: val.release_date,
      release_year,
      languages: val.languages,
      language_count,
      credits: val.credits.into_iter().map(|c| c.into()).collect(),
      credit_tags,
      credit_tag_count,
      duplicate_of: val.duplicate_of,
      duplicates: val.duplicates,
      is_duplicate,
      cover_image_url: val.cover_image_url,
      spotify_id: val.spotify_id,
    }
  }
}

impl TryFrom<&Vec<(String, String)>> for RedisAlbumReadModel {
  type Error = Error;

  fn try_from(values: &Vec<(String, String)>) -> Result<Self> {
    let json = values
      .first()
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
      .first()
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
    if let Some(text) = &self.text {
      ft_search_query.push_str(&format!("({}) ", escape_search_query_text(text)));
    }
    if let Some(exact_name) = &self.exact_name {
      ft_search_query.push_str(&get_tag_query("@name_tag", &vec![exact_name]));
    }
    if !self.include_duplicates.is_some_and(|b| b) {
      ft_search_query.push_str(&get_num_range_query("@is_duplicate", Some(0), Some(0)));
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
    ft_search_query.push_str(&get_tag_query("-@descriptor", &self.exclude_descriptors));
    ft_search_query.trim().to_string()
  }
}

impl AlbumEmbeddingSimilarirtySearchQuery {
  pub fn to_ft_search_query(&self) -> String {
    format!(
      "({})=>[KNN {} @{} $BLOB as distance]",
      self.filters.to_ft_search_query(),
      self.limit,
      embedding_json_key(&self.embedding_key)
    )
  }
}

pub struct RedisAlbumSearchIndex {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  version_manager: SearchIndexVersionManager,
  embedding_provider_interactor: Arc<EmbeddingProviderInteractor>,
}

const NAMESPACE: &str = "album";
const INDEX_VERSION: u32 = 8;

fn redis_key(file_name: &FileName) -> String {
  format!("{}:{}", NAMESPACE, file_name.to_string())
}

fn embedding_json_key(key: &str) -> String {
  let normalized_key = key.replace('-', "_");
  format!("embedding_{}", normalized_key)
}

fn embedding_json_path(key: &str) -> String {
  format!("$.{}", embedding_json_key(key))
}

impl RedisAlbumSearchIndex {
  fn get_schema(embedding_provider_interactor: &EmbeddingProviderInteractor) -> Vec<FtFieldSchema> {
    let mut schema = vec![
      FtFieldSchema::identifier("$.ascii_name")
        .as_attribute("ascii_name")
        .field_type(FtFieldType::Text)
        .weight(2.0),
      FtFieldSchema::identifier("$.file_name")
        .as_attribute("file_name")
        .field_type(FtFieldType::Tag),
      FtFieldSchema::identifier("$.artists[*].ascii_name")
        .as_attribute("artist_ascii_name")
        .field_type(FtFieldType::Text),
      FtFieldSchema::identifier("$.artists[*].file_name")
        .as_attribute("artist_file_name")
        .field_type(FtFieldType::Tag),
      FtFieldSchema::identifier("$.rating")
        .as_attribute("rating")
        .field_type(FtFieldType::Numeric),
      FtFieldSchema::identifier("$.rating_count")
        .as_attribute("rating_count")
        .field_type(FtFieldType::Numeric)
        .sortable(),
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
      FtFieldSchema::identifier("$.is_duplicate")
        .as_attribute("is_duplicate")
        .field_type(FtFieldType::Numeric),
      FtFieldSchema::identifier("$.name_tag")
        .as_attribute("name_tag")
        .field_type(FtFieldType::Tag),
    ];
    schema.extend(
      embedding_provider_interactor
        .providers
        .iter()
        .map(|(name, provider)| {
          FtFieldSchema::identifier(embedding_json_path(name))
            .as_attribute(embedding_json_key(name))
            .field_type(FtFieldType::Vector(Some(FtVectorFieldAlgorithm::Flat(
              FtFlatVectorFieldAttributes::new(
                FtVectorType::Float32,
                provider.dimensions(),
                FtVectorDistanceMetric::Cosine,
              ),
            ))))
        })
        .collect::<Vec<FtFieldSchema>>(),
    );
    schema
  }

  pub fn new(
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    embedding_provider_interactor: Arc<EmbeddingProviderInteractor>,
  ) -> Self {
    Self {
      version_manager: SearchIndexVersionManager::new(
        Arc::clone(&redis_connection_pool),
        INDEX_VERSION,
        "album-idx".to_string(),
      ),
      redis_connection_pool,
      embedding_provider_interactor,
    }
  }

  pub async fn setup_index(&self) -> Result<()> {
    self
      .version_manager
      .setup_index(
        FtCreateOptions::default()
          .on(FtIndexDataType::Json)
          .prefix(format!("{}:", NAMESPACE)),
        RedisAlbumSearchIndex::get_schema(&self.embedding_provider_interactor),
      )
      .await
  }

  fn index_name(&self) -> String {
    self.version_manager.latest_index_name()
  }

  pub async fn ensure_album_root(&self, file_name: &FileName) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection
      .json_get(redis_key(file_name), JsonGetOptions::default())
      .await?;
    if result.is_none() || result.is_some_and(|r| r == "{}") {
      connection
        .json_set(redis_key(file_name), "$", "{}", SetCondition::default())
        .await?;
    }
    Ok(())
  }

  async fn get_legacy_embeddings(&self, file_name: &FileName) -> Result<Vec<EmbeddingDocument>> {
    let result: Option<String> = self
      .redis_connection_pool
      .get()
      .await?
      .json_get(
        redis_key(file_name),
        JsonGetOptions::default().path("$.embeddings[*]"),
      )
      .await?;
    let embeddings = result
      .map(|r| serde_json::from_str::<Vec<EmbeddingDocument>>(&r))
      .transpose()?
      .unwrap_or_default();

    Ok(embeddings)
  }

  async fn delete_legacy_embeddings(&self, file_name: &FileName) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .json_del(redis_key(file_name), "$.embeddings")
      .await?;
    Ok(())
  }
}

#[async_trait]
impl AlbumSearchIndex for RedisAlbumSearchIndex {
  async fn get_embedding_keys(&self) -> Result<Vec<String>> {
    Ok(
      self
        .embedding_provider_interactor
        .providers
        .keys()
        .map(|provider| provider.to_string())
        .collect(),
    )
  }

  async fn put(&self, album: AlbumReadModel) -> Result<()> {
    let current_embedddings = self.get_embeddings(&album.file_name).await?;
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        redis_key(&album.file_name),
        "$",
        serde_json::to_string::<RedisAlbumReadModel>(&album.into())?,
        SetCondition::default(),
      )
      .await?;
    if !current_embedddings.is_empty() {
      for embedding in current_embedddings {
        self.put_embedding(embedding).await?;
      }
    }
    Ok(())
  }

  async fn put_many(&self, albums: Vec<AlbumReadModel>) -> Result<()> {
    for album in albums {
      if let Err(e) = self.put(album).await {
        warn!("failed to put album: {:?}", e);
      }
    }
    Ok(())
  }

  async fn delete(&self, file_name: &FileName) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection.del(redis_key(file_name)).await?;
    Ok(())
  }

  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection
      .json_get(redis_key(file_name), JsonGetOptions::default())
      .await?;
    let record = result
      .map(|r| serde_json::from_str::<RedisAlbumReadModel>(&r))
      .transpose()?
      .map(|r| r.into());

    Ok(record)
  }

  #[instrument(skip(self))]
  async fn search(
    &self,
    query: &AlbumSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<AlbumSearchResult> {
    let limit = pagination.and_then(|p| p.limit).unwrap_or(100000);
    let offset = pagination.and_then(|p| p.offset).unwrap_or(0);

    let result = self
      .redis_connection_pool
      .get()
      .await?
      .ft_search(
        self.index_name(),
        query.to_ft_search_query(),
        FtSearchOptions::default().limit(offset, limit)._return([
          FtSearchReturnAttribute::identifier("$.name"),
          FtSearchReturnAttribute::identifier("$.file_name"),
          FtSearchReturnAttribute::identifier("$.rating"),
          FtSearchReturnAttribute::identifier("$.rating_count"),
          FtSearchReturnAttribute::identifier("$.artists"),
          FtSearchReturnAttribute::identifier("$.primary_genres"),
          FtSearchReturnAttribute::identifier("$.secondary_genres"),
          FtSearchReturnAttribute::identifier("$.descriptors"),
          FtSearchReturnAttribute::identifier("$.tracks"),
          FtSearchReturnAttribute::identifier("$.release_date"),
          FtSearchReturnAttribute::identifier("$.languages"),
          FtSearchReturnAttribute::identifier("$.credits"),
          FtSearchReturnAttribute::identifier("$.duplicate_of"),
          FtSearchReturnAttribute::identifier("$.duplicates"),
          FtSearchReturnAttribute::identifier("$.cover_image_url"),
          FtSearchReturnAttribute::identifier("$.spotify_id"),
        ]),
      )
      .await?;

    let mut albums = Vec::with_capacity(result.results.len());
    for item in result.results {
      let mut album_builder = AlbumReadModelBuilder::default();
      for (key, value) in item.values {
        match key.as_str() {
          "$.name" => {
            album_builder.name(value);
          }
          "$.file_name" => {
            album_builder.file_name(FileName::try_from(value)?);
          }
          "$.rating" => {
            album_builder.rating(value.parse()?);
          }
          "$.rating_count" => {
            album_builder.rating_count(value.parse()?);
          }
          "$.artists" => {
            album_builder.artists(serde_json::from_str(value.as_str())?);
          }
          "$.primary_genres" => {
            album_builder.primary_genres(serde_json::from_str(value.as_str())?);
          }
          "$.secondary_genres" => {
            album_builder.secondary_genres(serde_json::from_str(value.as_str())?);
          }
          "$.descriptors" => {
            album_builder.descriptors(serde_json::from_str(value.as_str())?);
          }
          "$.tracks" => {
            album_builder.tracks(serde_json::from_str(value.as_str())?);
          }
          "$.release_date" => {
            match value.as_str() {
              "" => album_builder.release_date(None),
              _ => album_builder
                .release_date(Some(NaiveDate::parse_from_str(value.as_str(), "%Y-%m-%d")?)),
            };
          }
          "$.languages" => {
            album_builder.languages(serde_json::from_str(value.as_str())?);
          }
          "$.credits" => {
            album_builder.credits(serde_json::from_str(value.as_str())?);
          }
          "$.duplicate_of" => {
            match value.as_str() {
              "" => album_builder.duplicate_of(None),
              _ => album_builder.duplicate_of(Some(FileName::try_from(value)?)),
            };
          }
          "$.duplicates" => {
            album_builder.duplicates(serde_json::from_str(value.as_str())?);
          }
          "$.cover_image_url" => {
            match value.as_str() {
              "" => album_builder.cover_image_url(None),
              _ => album_builder.cover_image_url(Some(value)),
            };
          }
          "$.spotify_id" => {
            match value.as_str() {
              "" => album_builder.spotify_id(None),
              _ => album_builder.spotify_id(Some(value)),
            };
          }
          _ => {}
        };
      }
      albums.push(album_builder.build()?);
    }

    Ok(AlbumSearchResult {
      albums,
      total: result.total_results,
    })
  }

  #[instrument(skip_all)]
  async fn put_embedding(&self, embedding: EmbeddingDocument) -> Result<()> {
    self.ensure_album_root(&embedding.file_name).await?;
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        redis_key(&embedding.file_name),
        embedding_json_path(&embedding.key),
        serde_json::to_string(&embedding.embedding)?,
        SetCondition::default(),
      )
      .await?;
    match self.delete_legacy_embeddings(&embedding.file_name).await {
      Ok(_) => {}
      Err(e) => {
        tracing::warn!("failed to delete legacy embeddings: {:?}", e);
      }
    };
    Ok(())
  }

  async fn put_many_embeddings(&self, docs: Vec<EmbeddingDocument>) -> Result<()> {
    stream::iter(docs)
      .map(Ok)
      .try_for_each_concurrent(250, |embedding| async {
        self.put_embedding(embedding).await
      })
      .await
  }

  async fn get_embeddings(&self, file_name: &FileName) -> Result<Vec<EmbeddingDocument>> {
    let legacy_embeddings = self.get_legacy_embeddings(file_name).await?;
    let embeddings = join_all(self.embedding_provider_interactor.providers.keys().map(
      |name| async {
        let embedding = match self.find_embedding(file_name, name).await {
          Ok(embedding) => embedding.map(|embedding| EmbeddingDocument {
            file_name: file_name.clone(),
            key: name.to_string(),
            embedding: embedding.embedding,
          }),
          Err(_) => None,
        };
        Ok(embedding)
      },
    ))
    .await
    .into_iter()
    .filter_map(|result| result.transpose())
    .collect::<Result<Vec<EmbeddingDocument>>>()?;

    Ok([legacy_embeddings, embeddings].concat())
  }

  async fn find_many_embeddings(
    &self,
    file_names: Vec<FileName>,
    key: &str,
  ) -> Result<Vec<EmbeddingDocument>> {
    let embeddings = join_all(
      file_names
        .iter()
        .map(|file_name| self.find_embedding(file_name, key)),
    )
    .await
    .into_iter()
    .filter_map(|result| result.transpose())
    .collect::<Result<Vec<EmbeddingDocument>>>()?;
    Ok(embeddings)
  }

  async fn delete_embedding(&self, file_name: &FileName, key: &str) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .json_del(redis_key(file_name), embedding_json_path(key))
      .await?;
    Ok(())
  }

  async fn find_embedding(
    &self,
    file_name: &FileName,
    key: &str,
  ) -> Result<Option<EmbeddingDocument>> {
    let key = embedding_json_key(key);
    let result: Result<Option<String>, rustis::Error> = self
      .redis_connection_pool
      .get()
      .await?
      .json_get(
        redis_key(file_name),
        JsonGetOptions::default().path(key.clone()),
      )
      .await;

    let result = match result {
      Ok(r) => r,
      Err(e)
        if e
          .to_string()
          .contains(&format!("ERR Path '$.{}' does not exist", key)) =>
      {
        return Ok(None)
      }
      Err(e) => return Err(e.into()),
    };

    let embedding = result
      .map(|r| serde_json::from_str::<Vec<f32>>(&r))
      .transpose()?
      .map(|embedding| EmbeddingDocument {
        file_name: file_name.clone(),
        key: key.to_string(),
        embedding,
      });
    Ok(embedding)
  }

  #[instrument(skip(self))]
  async fn embedding_similarity_search(
    &self,
    query: &AlbumEmbeddingSimilarirtySearchQuery,
  ) -> Result<Vec<(AlbumReadModel, f32)>> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_search(
        self.index_name(),
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
          .first()
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
}
