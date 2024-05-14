use crate::{
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{ParsedAlbum, ParsedArtistReference, ParsedCredit, ParsedTrack},
  proto,
};
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

impl AlbumReadModelArtist {
  pub fn ascii_name(&self) -> String {
    unidecode(&self.name)
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

  pub fn from_parsed_album(file_name: &FileName, parsed_album: ParsedAlbum) -> Self {
    Self {
      name: parsed_album.name.clone(),
      file_name: file_name.clone(),
      rating: parsed_album.rating,
      rating_count: parsed_album.rating_count,
      artists: parsed_album
        .artists
        .iter()
        .map(AlbumReadModelArtist::from)
        .collect::<Vec<AlbumReadModelArtist>>(),
      primary_genres: parsed_album.primary_genres.clone(),
      secondary_genres: parsed_album.secondary_genres.clone(),
      descriptors: parsed_album.descriptors.clone(),
      tracks: parsed_album
        .tracks
        .iter()
        .map(AlbumReadModelTrack::from)
        .collect::<Vec<AlbumReadModelTrack>>(),
      release_date: parsed_album.release_date,
      languages: parsed_album.languages.clone(),
      credits: parsed_album
        .credits
        .iter()
        .map(AlbumReadModelCredit::from)
        .collect::<Vec<AlbumReadModelCredit>>(),
      duplicates: vec![],
      duplicate_of: None,
      cover_image_url: parsed_album.cover_image_url,
    }
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

impl From<AlbumReadModel> for ParsedAlbum {
  fn from(album: AlbumReadModel) -> Self {
    Self {
      name: album.name,
      rating: album.rating,
      rating_count: album.rating_count,
      artists: album
        .artists
        .iter()
        .map(|artist| ParsedArtistReference {
          name: artist.name.clone(),
          file_name: artist.file_name.clone(),
        })
        .collect::<Vec<ParsedArtistReference>>(),
      primary_genres: album.primary_genres,
      secondary_genres: album.secondary_genres,
      descriptors: album.descriptors,
      tracks: album
        .tracks
        .iter()
        .map(|track| ParsedTrack {
          name: track.name.clone(),
          duration_seconds: track.duration_seconds,
          rating: track.rating,
          position: track.position.clone(),
        })
        .collect::<Vec<ParsedTrack>>(),
      release_date: album.release_date,
      languages: album.languages,
      credits: album
        .credits
        .iter()
        .map(|credit| ParsedCredit {
          artist: ParsedArtistReference {
            name: credit.artist.name.clone(),
            file_name: credit.artist.file_name.clone(),
          },
          roles: credit.roles.clone(),
        })
        .collect::<Vec<ParsedCredit>>(),
      cover_image_url: album.cover_image_url,
    }
  }
}
