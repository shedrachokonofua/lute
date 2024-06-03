use crate::{
  albums::album_read_model::AlbumReadModel, files::file_metadata::file_name::FileName,
  helpers::item_with_factor::ItemWithFactor,
};
use chrono::Datelike;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unidecode::unidecode;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct ArtistReadModelCredit {
  pub album_file_name: FileName,
  pub roles: Vec<String>,
}

pub struct ArtistReadModel {
  pub name: String,
  pub file_name: FileName,
  pub album_file_names: Vec<FileName>,
  pub credits: Vec<ArtistReadModelCredit>,
}

impl ArtistReadModel {
  pub fn ascii_name(&self) -> String {
    unidecode(&self.name)
  }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct ArtistAlbumSummary {
  pub album_count: u32,
  pub average_rating: f32,
  pub total_rating_count: u32,
  pub min_year: u32,
  pub max_year: u32,
  pub primary_genres: Vec<ItemWithFactor>,
  pub secondary_genres: Vec<ItemWithFactor>,
  pub descriptors: Vec<ItemWithFactor>,
}

impl ArtistAlbumSummary {
  pub fn from_albums(albums: Vec<&AlbumReadModel>) -> Self {
    let album_count = albums.len() as u32;
    let total_rating_count = albums.iter().map(|a| a.rating_count).sum();
    let average_rating = if total_rating_count > 0 {
      albums
        .iter()
        .map(|a| a.rating * a.rating_count as f32)
        .sum::<f32>()
        / total_rating_count as f32
    } else {
      0.0
    };
    let min_year = albums
      .iter()
      .filter_map(|a| a.release_date.map(|d| d.year()))
      .min()
      .unwrap_or(0) as u32;
    let max_year = albums
      .iter()
      .filter_map(|a| a.release_date.map(|d| d.year()))
      .max()
      .unwrap_or(0) as u32;

    let mut primary_genres = HashMap::new();
    let mut secondary_genres = HashMap::new();
    let mut descriptors = HashMap::new();
    for album in albums {
      for genre in &album.primary_genres {
        let entry = primary_genres
          .entry(genre.clone())
          .or_insert_with(|| ItemWithFactor {
            item: genre.clone(),
            factor: 0,
          });
        entry.factor += 1;
      }
      for genre in &album.secondary_genres {
        let entry = secondary_genres
          .entry(genre.clone())
          .or_insert_with(|| ItemWithFactor {
            item: genre.clone(),
            factor: 0,
          });
        entry.factor += 1;
      }
      for descriptor in &album.descriptors {
        let entry = descriptors
          .entry(descriptor.clone())
          .or_insert_with(|| ItemWithFactor {
            item: descriptor.clone(),
            factor: 0,
          });
        entry.factor += 1;
      }
    }
    let primary_genres = primary_genres.values().cloned().collect();
    let secondary_genres = secondary_genres.values().cloned().collect();
    let descriptors = descriptors.values().cloned().collect();
    Self {
      album_count,
      average_rating,
      total_rating_count,
      min_year,
      max_year,
      primary_genres,
      secondary_genres,
      descriptors,
    }
  }
}

#[derive(Debug, PartialEq, Builder, Serialize, Deserialize, Clone, Default)]
#[builder(default)]
pub struct ArtistOverview {
  pub name: String,
  pub file_name: FileName,
  pub credit_roles: Vec<ItemWithFactor>,
  pub album_summary: ArtistAlbumSummary,
  pub credited_album_summary: ArtistAlbumSummary,
}

impl ArtistOverview {
  pub fn ascii_name(&self) -> String {
    unidecode(&self.name)
  }

  pub fn new(artist: ArtistReadModel, albums: &HashMap<FileName, AlbumReadModel>) -> Self {
    let mut credit_roles = HashMap::new();
    for credit in &artist.credits {
      for role in &credit.roles {
        let entry = credit_roles
          .entry(role.clone())
          .or_insert_with(|| ItemWithFactor {
            item: role.clone(),
            factor: 0,
          });
        entry.factor += 1;
      }
    }
    let credit_roles = credit_roles.values().cloned().collect::<Vec<_>>();

    let album_summary = ArtistAlbumSummary::from_albums(
      artist
        .album_file_names
        .iter()
        .filter_map(|file_name| albums.get(file_name))
        .collect(),
    );
    let credited_album_summary = ArtistAlbumSummary::from_albums(
      artist
        .credits
        .iter()
        .flat_map(|credit| albums.get(&credit.album_file_name))
        .collect(),
    );

    Self {
      name: artist.name,
      file_name: artist.file_name,
      credit_roles,
      album_summary,
      credited_album_summary,
    }
  }
}
