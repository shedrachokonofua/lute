use super::profile::{Profile, ProfileId};
use crate::{
  albums::album_read_model::AlbumReadModel,
  files::file_metadata::file_name::FileName,
  helpers::math::{desc_sort_by, median},
};
use chrono::Datelike;
use rayon::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, iter::repeat};
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize, Clone, Default, Eq)]
pub struct ItemWithFactor {
  pub item: String,
  pub factor: u32,
}

impl Ord for ItemWithFactor {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.factor.cmp(&other.factor)
  }
}

impl PartialOrd for ItemWithFactor {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for ItemWithFactor {
  fn eq(&self, other: &Self) -> bool {
    self.factor == other.factor
  }
}

fn desc_sort_by_factor(values: &mut [ItemWithFactor]) {
  desc_sort_by(values, |item| item.factor as f32);
}

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
    let album_read_models_map = album_read_models
      .into_par_iter()
      .map(|album_read_model| (album_read_model.file_name.clone(), album_read_model))
      .collect::<HashMap<FileName, &AlbumReadModel>>();
    let mut artists_map: HashMap<String, u32> = HashMap::new();
    let mut primary_genres_map: HashMap<String, u32> = HashMap::new();
    let mut secondary_genres_map: HashMap<String, u32> = HashMap::new();
    let mut descriptors_map: HashMap<String, u32> = HashMap::new();
    let mut years_map: HashMap<u32, u32> = HashMap::new();
    let mut decades_map: HashMap<u32, u32> = HashMap::new();
    let mut credit_tags_map: HashMap<String, u32> = HashMap::new();
    let mut ratings: Vec<f32> = Vec::new();
    let mut indexed_album_count = 0;

    for (file_name, factor) in &self.albums {
      let album = album_read_models_map.get(file_name);
      if album.is_none() {
        continue;
      }
      let album = album.unwrap();
      indexed_album_count += 1;

      for artist in &album.artists {
        artists_map
          .entry(artist.name.clone())
          .and_modify(|c| *c += factor)
          .or_insert(*factor);
      }

      for genre in &album.primary_genres {
        primary_genres_map
          .entry(genre.clone())
          .and_modify(|c| *c += factor)
          .or_insert(*factor);
      }

      for genre in &album.secondary_genres {
        secondary_genres_map
          .entry(genre.clone())
          .and_modify(|c| *c += factor)
          .or_insert(*factor);
      }

      for descriptor in &album.descriptors {
        descriptors_map
          .entry(descriptor.clone())
          .and_modify(|c| *c += factor)
          .or_insert(*factor);
      }

      for tag in &album.credit_tags() {
        credit_tags_map
          .entry(tag.clone())
          .and_modify(|c| *c += factor)
          .or_insert(*factor);
      }

      if let Some(release_date) = album.release_date {
        let year = release_date.year() as u32;
        years_map
          .entry(year)
          .and_modify(|c| *c += factor)
          .or_insert(*factor);
        let decade = year - (year % 10);
        decades_map
          .entry(decade)
          .and_modify(|c| *c += factor)
          .or_insert(*factor);
      }

      ratings.append(
        &mut repeat(album.rating)
          .take(*factor as usize)
          .collect::<Vec<_>>(),
      );
    }

    let average_rating = ratings.iter().sum::<f32>() / ratings.len() as f32;

    let mut artists = artists_map
      .iter()
      .map(|(item, factor)| ItemWithFactor {
        item: item.clone(),
        factor: *factor,
      })
      .collect::<Vec<ItemWithFactor>>();
    desc_sort_by_factor(&mut artists);

    let mut years = years_map
      .iter()
      .map(|(item, factor)| ItemWithFactor {
        item: item.to_string(),
        factor: *factor,
      })
      .collect::<Vec<ItemWithFactor>>();
    desc_sort_by_factor(&mut years);

    let median_year = median(
      years
        .iter()
        .flat_map(|item| {
          repeat(item.item.parse::<f32>().unwrap())
            .take(item.factor as usize)
            .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>(),
    )
    .round() as u32;

    let mut primary_genres = primary_genres_map
      .iter()
      .map(|(item, factor)| ItemWithFactor {
        item: item.clone(),
        factor: *factor,
      })
      .collect::<Vec<ItemWithFactor>>();
    desc_sort_by_factor(&mut primary_genres);

    let mut secondary_genres = secondary_genres_map
      .iter()
      .map(|(item, factor)| ItemWithFactor {
        item: item.clone(),
        factor: *factor,
      })
      .collect::<Vec<ItemWithFactor>>();
    desc_sort_by_factor(&mut secondary_genres);

    let mut descriptors = descriptors_map
      .iter()
      .map(|(item, factor)| ItemWithFactor {
        item: item.clone(),
        factor: *factor,
      })
      .collect::<Vec<ItemWithFactor>>();
    desc_sort_by_factor(&mut descriptors);

    let mut credit_tags = credit_tags_map
      .iter()
      .map(|(item, factor)| ItemWithFactor {
        item: item.clone(),
        factor: *factor,
      })
      .collect::<Vec<ItemWithFactor>>();
    desc_sort_by_factor(&mut credit_tags);

    let mut decades = decades_map
      .iter()
      .map(|(item, factor)| ItemWithFactor {
        item: item.to_string(),
        factor: *factor,
      })
      .collect::<Vec<ItemWithFactor>>();
    desc_sort_by_factor(&mut decades);

    ProfileSummary {
      id: self.id.clone(),
      name: self.name.clone(),
      album_count: self.albums.len() as u32,
      indexed_album_count,
      average_rating,
      median_year,
      artists,
      primary_genres,
      secondary_genres,
      descriptors,
      credit_tags,
      years,
      decades,
    }
  }
}
