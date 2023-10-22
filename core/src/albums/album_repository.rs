use crate::{files::file_metadata::file_name::FileName, proto};
use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use data_encoding::BASE64;
use derive_builder::Builder;
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
      languages: val.languages,
      cover_image_url: val.cover_image_url,
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
pub struct AlbumReadModelCredit {
  pub artist: AlbumReadModelArtist,
  pub roles: Vec<String>,
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
  pub languages: Vec<String>,
  pub credits: Vec<AlbumReadModelCredit>,
  pub duplicate_of: Option<FileName>,
  pub duplicates: Vec<FileName>,
  pub cover_image_url: Option<String>,
}

impl AlbumReadModel {
  pub fn credit_tags(&self) -> Vec<String> {
    self
      .credits
      .iter()
      .flat_map(|credit| {
        credit.roles.iter().map(|role| {
          format!(
            "{}:{}",
            credit.artist.file_name.to_string(),
            role.to_lowercase().replace(' ', "_")
          )
        })
      })
      .collect::<Vec<String>>()
  }

  pub fn to_sha256(&self) -> Result<String> {
    let hash = Sha256::digest(serde_json::to_string(&self)?.as_bytes());
    Ok(BASE64.encode(&hash).to_string())
  }
}

pub struct GenreAggregate {
  pub name: String,
  pub primary_genre_count: u32,
  pub secondary_genre_count: u32,
}

pub struct ItemAndCount {
  pub name: String,
  pub count: u32,
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

impl From<&ItemAndCount> for proto::DescriptorAggregate {
  fn from(val: &ItemAndCount) -> Self {
    proto::DescriptorAggregate {
      name: val.name.clone(),
      count: val.count,
    }
  }
}

impl From<&ItemAndCount> for proto::LanguageAggregate {
  fn from(val: &ItemAndCount) -> Self {
    proto::LanguageAggregate {
      name: val.name.clone(),
      count: val.count,
    }
  }
}

#[derive(Default, Builder, Debug)]
#[builder(setter(into), default)]
pub struct AlbumSearchQuery {
  pub exact_name: Option<String>,
  pub include_file_names: Vec<FileName>,
  pub exclude_file_names: Vec<FileName>,
  pub include_artists: Vec<String>,
  pub exclude_artists: Vec<String>,
  pub include_primary_genres: Vec<String>,
  pub exclude_primary_genres: Vec<String>,
  pub include_secondary_genres: Vec<String>,
  pub exclude_secondary_genres: Vec<String>,
  pub include_languages: Vec<String>,
  pub exclude_languages: Vec<String>,
  pub include_descriptors: Vec<String>,
  pub min_primary_genre_count: Option<usize>,
  pub min_secondary_genre_count: Option<usize>,
  pub min_descriptor_count: Option<usize>,
  pub min_release_year: Option<u32>,
  pub max_release_year: Option<u32>,
  pub include_duplicates: Option<bool>,
}

impl TryFrom<proto::AlbumSearchQuery> for AlbumSearchQuery {
  type Error = anyhow::Error;

  fn try_from(value: proto::AlbumSearchQuery) -> Result<Self> {
    Ok(AlbumSearchQuery {
      exact_name: value.exact_name,
      include_file_names: value
        .include_file_names
        .into_iter()
        .map(|file_name| FileName::try_from(file_name).map_err(|e| anyhow::Error::msg(e)))
        .collect::<Result<Vec<FileName>>>()?,
      exclude_file_names: value
        .exclude_file_names
        .into_iter()
        .map(|file_name| FileName::try_from(file_name).map_err(|e| anyhow::Error::msg(e)))
        .collect::<Result<Vec<FileName>>>()?,
      include_artists: value.include_artists,
      exclude_artists: value.exclude_artists,
      include_primary_genres: value.include_primary_genres,
      exclude_primary_genres: value.exclude_primary_genres,
      include_secondary_genres: value.include_secondary_genres,
      exclude_secondary_genres: value.exclude_secondary_genres,
      include_languages: value.include_languages,
      exclude_languages: value.exclude_languages,
      include_descriptors: value.include_descriptors,
      min_primary_genre_count: value.min_primary_genre_count.map(|i| i as usize),
      min_secondary_genre_count: value.min_secondary_genre_count.map(|i| i as usize),
      min_descriptor_count: value.min_descriptor_count.map(|i| i as usize),
      min_release_year: value.min_release_year.map(|i| i as u32),
      max_release_year: value.max_release_year.map(|i| i as u32),
      include_duplicates: value.include_duplicates,
    })
  }
}

#[derive(Debug)]
pub struct SearchPagination {
  pub offset: Option<usize>,
  pub limit: Option<usize>,
}

impl TryFrom<proto::SearchPagination> for SearchPagination {
  type Error = anyhow::Error;

  fn try_from(value: proto::SearchPagination) -> Result<Self> {
    Ok(SearchPagination {
      offset: value.offset.map(|i| i as usize),
      limit: value.limit.map(|i| i as usize),
    })
  }
}

#[derive(Debug)]
pub struct AlbumSearchResult {
  pub albums: Vec<AlbumReadModel>,
  pub total: usize,
}

#[derive(Debug)]
pub struct SimilarAlbumsQuery {
  pub embedding: Vec<f32>,
  pub embedding_key: String,
  pub filters: AlbumSearchQuery,
  pub limit: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct AlbumEmbedding {
  pub file_name: FileName,
  pub key: String,
  pub embedding: Vec<f32>,
}

pub fn embedding_to_bytes(embedding: &Vec<f32>) -> Vec<u8> {
  embedding
    .iter()
    .flat_map(|f| f.to_ne_bytes().to_vec())
    .collect()
}

impl AlbumEmbedding {
  pub fn embedding_bytes(&self) -> Vec<u8> {
    embedding_to_bytes(&self.embedding)
  }
}

#[async_trait]
pub trait AlbumRepository {
  async fn put(&self, album: AlbumReadModel) -> Result<()>;
  async fn delete(&self, file_name: &FileName) -> Result<()>;
  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>>;
  async fn exists(&self, file_name: &FileName) -> Result<bool>;
  async fn get_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>>;
  async fn search(
    &self,
    query: &AlbumSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<AlbumSearchResult>;
  async fn get_aggregated_genres(&self) -> Result<Vec<GenreAggregate>>;
  async fn get_aggregated_descriptors(&self) -> Result<Vec<ItemAndCount>>;
  async fn get_aggregated_languages(&self) -> Result<Vec<ItemAndCount>>;
  async fn get_embeddings(&self, file_name: &FileName) -> Result<Vec<AlbumEmbedding>>;
  async fn find_many_embeddings(
    &self,
    file_name: Vec<FileName>,
    key: &str,
  ) -> Result<Vec<AlbumEmbedding>>;
  async fn find_embedding(&self, file_name: &FileName, key: &str)
    -> Result<Option<AlbumEmbedding>>;
  async fn put_embedding(&self, embedding: &AlbumEmbedding) -> Result<()>;
  async fn delete_embedding(&self, file_name: &FileName, key: &str) -> Result<()>;
  async fn find_similar_albums(
    &self,
    query: &SimilarAlbumsQuery,
  ) -> Result<Vec<(AlbumReadModel, f32)>>;
  async fn get_embedding_keys(&self) -> Result<Vec<String>>;
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
