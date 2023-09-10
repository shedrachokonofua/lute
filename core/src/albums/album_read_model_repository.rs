use crate::{files::file_metadata::file_name::FileName, proto};
use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use derive_builder::Builder;
use serde_derive::{Deserialize, Serialize};
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
}

#[async_trait]
pub trait AlbumReadModelRepository {
  async fn put(&self, album: AlbumReadModel) -> Result<()>;
  async fn delete(&self, file_name: &FileName) -> Result<()>;
  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>>;
  async fn exists(&self, file_name: &FileName) -> Result<bool>;
  async fn get_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>>;
  async fn search(&self, query: &AlbumSearchQuery) -> Result<Vec<AlbumReadModel>>;
  async fn get_aggregated_genres(&self) -> Result<Vec<GenreAggregate>>;
  async fn get_aggregated_descriptors(&self) -> Result<Vec<ItemAndCount>>;
  async fn get_aggregated_languages(&self) -> Result<Vec<ItemAndCount>>;

  async fn get(&self, file_name: &FileName) -> Result<AlbumReadModel> {
    let record = self.find(file_name).await?;
    match record {
      Some(record) => Ok(record),
      None => anyhow::bail!("Album does not exist"),
    }
  }
}
