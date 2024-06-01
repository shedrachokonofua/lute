use super::album_read_model::AlbumReadModel;
use crate::{files::file_metadata::file_name::FileName, helpers::item_with_factor::ItemWithFactor};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct ArtistOverviewCreditAlbum {
  pub name: String,
  pub file_name: FileName,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct ArtistOverviewCredit {
  pub album: ArtistOverviewCreditAlbum,
  pub roles: Vec<String>,
}

#[derive(Debug, PartialEq, Builder, Serialize, Deserialize, Clone, Default)]
#[builder(default)]
pub struct ArtistOverview {
  pub name: String,
  pub file_name: FileName,
  pub average_rating: f32,
  pub total_rating_count: u32,
  pub min_year: u32,
  pub max_year: u32,
  pub primary_genres: Vec<ItemWithFactor>,
  pub secondary_genres: Vec<ItemWithFactor>,
  pub descriptors: Vec<ItemWithFactor>,
  pub credit_roles: Vec<ItemWithFactor>,
  pub albums: Vec<AlbumReadModel>,
  pub credits: Vec<ArtistOverviewCredit>,
}
