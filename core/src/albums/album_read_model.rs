use crate::{files::file_metadata::file_name::FileName, proto};
use anyhow::Result;
use chrono::NaiveDate;
use data_encoding::BASE64;
use derive_builder::Builder;
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unidecode::unidecode;

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
pub struct AlbumReadModelCredit {
  pub artist: AlbumReadModelArtist,
  pub roles: Vec<String>,
}

#[derive(Debug, PartialEq, Builder, Serialize, Deserialize, Clone, Default)]
#[builder(default)]
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

  pub fn ascii_name(&self) -> String {
    unidecode(&self.name)
  }
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
      duplicate_of: val.duplicate_of.map(|file_name| file_name.to_string()),
      duplicates: val
        .duplicates
        .into_iter()
        .map(|file_name| file_name.to_string())
        .collect(),
    }
  }
}
