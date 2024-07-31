use super::profile::{Profile, ProfileId};
use crate::{
  albums::{album_collection_summary::AlbumCollectionSummary, album_read_model::AlbumReadModel},
  helpers::item_with_factor::ItemWithFactor,
};
use serde_derive::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProfileSummary {
  pub id: ProfileId,
  pub name: String,
  pub album_count: u32,
  pub indexed_album_count: u32,
  pub average_rating: f32,
  pub median_year: u32,
  pub artists: Vec<ItemWithFactor>,
  pub primary_genres: Vec<ItemWithFactor>,
  pub secondary_genres: Vec<ItemWithFactor>,
  pub descriptors: Vec<ItemWithFactor>,
  pub years: Vec<ItemWithFactor>,
  pub decades: Vec<ItemWithFactor>,
  pub credit_tags: Vec<ItemWithFactor>,
}

impl Profile {
  #[instrument(skip_all, fields(id = %self.id.to_string(), len = album_read_models.len()))]
  pub fn summarize(&self, album_read_models: &[AlbumReadModel]) -> ProfileSummary {
    let indexed_album_count = album_read_models.len() as u32;
    let collection_sumary = AlbumCollectionSummary::new(album_read_models, &self.albums);

    ProfileSummary {
      id: self.id.clone(),
      name: self.name.clone(),
      album_count: self.albums.len() as u32,
      indexed_album_count,
      average_rating: collection_sumary.average_rating,
      median_year: collection_sumary.median_year,
      artists: collection_sumary.artists,
      primary_genres: collection_sumary.primary_genres,
      secondary_genres: collection_sumary.secondary_genres,
      descriptors: collection_sumary.descriptors,
      years: collection_sumary.years,
      decades: collection_sumary.decades,
      credit_tags: collection_sumary.credit_tags,
    }
  }
}
