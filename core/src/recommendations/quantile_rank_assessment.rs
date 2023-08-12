use super::{
  quantile_rank_interactor::QuantileRankAlbumAssessmentSettings, types::AlbumAssessment,
};
use crate::{
  albums::album_read_model_repository::AlbumReadModel,
  helpers::{math::default_if_zero, quantile_rank::QuantileRanking},
  profile::{profile::Profile, profile_summary::ItemWithFactor},
};
use anyhow::{anyhow, Result};
use num_traits::Zero;
use ordered_float::OrderedFloat;
use std::collections::HashMap;
use tracing::{instrument, warn};

#[instrument(skip(items))]
fn create_item_with_factor_map(items: &[ItemWithFactor]) -> HashMap<String, ItemWithFactor> {
  items
    .iter()
    .map(|item| (item.item.clone(), item.clone()))
    .collect::<HashMap<String, ItemWithFactor>>()
}

fn calculate_average_rank(
  ranking: &QuantileRanking<ItemWithFactor>,
  profile_tags_map: &HashMap<String, ItemWithFactor>,
  album_tags: &[String],
  novelty_score: f64,
) -> Result<f64> {
  if album_tags.is_empty() {
    return Ok(novelty_score);
  }

  let ranks = album_tags
    .iter()
    .map(|tag: &String| match profile_tags_map.get(tag) {
      Some(item) => default_if_zero(ranking.get_rank(item), novelty_score),
      None => novelty_score,
    })
    .collect::<Vec<f64>>();

  let rank = ranks.iter().sum::<f64>() / ranks.len() as f64;

  if rank.is_nan() {
    warn!("rank is NaN");
  }

  Ok(rank)
}

pub struct QuantileRankAlbumAssessmentContext {
  primary_genre_ranking: QuantileRanking<ItemWithFactor>,
  secondary_genre_ranking: QuantileRanking<ItemWithFactor>,
  descriptor_ranking: QuantileRanking<ItemWithFactor>,
  rating_ranking: QuantileRanking<OrderedFloat<f32>>,
  rating_count_ranking: QuantileRanking<u32>,
  descriptor_count_ranking: QuantileRanking<u32>,
  credit_tag_ranking: QuantileRanking<ItemWithFactor>,
  settings: QuantileRankAlbumAssessmentSettings,
  primary_genre_summary_map: HashMap<String, ItemWithFactor>,
  secondary_genre_summary_map: HashMap<String, ItemWithFactor>,
  descriptor_summary_map: HashMap<String, ItemWithFactor>,
  credit_tag_summary_map: HashMap<String, ItemWithFactor>,
}

impl QuantileRankAlbumAssessmentContext {
  pub fn new(
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Self {
    let profile_summary = profile.summarize(profile_albums);

    Self {
      settings,
      primary_genre_ranking: QuantileRanking::new(&profile_summary.primary_genres),
      secondary_genre_ranking: QuantileRanking::new(&profile_summary.secondary_genres),
      descriptor_ranking: QuantileRanking::new(&profile_summary.descriptors),
      rating_ranking: QuantileRanking::new(
        &profile_albums
          .iter()
          .map(|album| OrderedFloat(album.rating))
          .collect::<Vec<_>>(),
      ),
      rating_count_ranking: QuantileRanking::new(
        &profile_albums
          .iter()
          .map(|album| album.rating_count)
          .collect::<Vec<_>>(),
      ),
      descriptor_count_ranking: QuantileRanking::new(
        &profile_albums
          .iter()
          .map(|album| album.descriptor_count)
          .collect::<Vec<_>>(),
      ),
      credit_tag_ranking: QuantileRanking::new(&profile_summary.credit_tags),
      primary_genre_summary_map: create_item_with_factor_map(&profile_summary.primary_genres),
      secondary_genre_summary_map: create_item_with_factor_map(&profile_summary.secondary_genres),
      descriptor_summary_map: create_item_with_factor_map(&profile_summary.descriptors),
      credit_tag_summary_map: create_item_with_factor_map(&profile_summary.credit_tags),
    }
  }

  pub fn assess(&self, album: &AlbumReadModel) -> Result<AlbumAssessment> {
    let average_primary_genre_rank = if !self.settings.primary_genre_weight.is_zero() {
      calculate_average_rank(
        &self.primary_genre_ranking,
        &self.primary_genre_summary_map,
        &album.primary_genres,
        self.settings.novelty_score,
      )?
    } else {
      0.0
    };

    let average_secondary_genre_rank = if !self.settings.secondary_genre_weight.is_zero() {
      calculate_average_rank(
        &self.secondary_genre_ranking,
        &self.secondary_genre_summary_map,
        &album.secondary_genres,
        self.settings.novelty_score,
      )?
    } else {
      0.0
    };

    let average_descriptor_rank = if !self.settings.descriptor_weight.is_zero() {
      calculate_average_rank(
        &self.descriptor_ranking,
        &self.descriptor_summary_map,
        &album.descriptors,
        self.settings.novelty_score,
      )?
    } else {
      0.0
    };

    let average_credit_tag_rank = if !self.settings.credit_tag_weight.is_zero() {
      calculate_average_rank(
        &self.credit_tag_ranking,
        &self.credit_tag_summary_map,
        &album.credit_tags,
        0.1,
      )?
    } else {
      0.0
    };

    let rating_rank = if !self.settings.rating_weight.is_zero() {
      default_if_zero(
        self.rating_ranking.get_rank(&OrderedFloat(album.rating)),
        self.settings.novelty_score,
      )
    } else {
      0.0
    };

    let rating_count_rank = if !self.settings.rating_count_weight.is_zero() {
      default_if_zero(
        self.rating_count_ranking.get_rank(&album.rating_count),
        self.settings.novelty_score,
      )
    } else {
      0.0
    };

    let descriptor_count_rank = if !self.settings.descriptor_count_weight.is_zero() {
      default_if_zero(
        self
          .descriptor_count_ranking
          .get_rank(&album.descriptor_count),
        self.settings.novelty_score,
      )
    } else {
      0.0
    };

    let mut ranks = vec![average_primary_genre_rank; self.settings.primary_genre_weight as usize];
    ranks.append(&mut vec![
      average_secondary_genre_rank;
      self.settings.secondary_genre_weight as usize
    ]);
    ranks.append(&mut vec![
      average_descriptor_rank;
      self.settings.descriptor_weight as usize
    ]);
    ranks.append(&mut vec![rating_rank; self.settings.rating_weight as usize]);
    ranks.append(&mut vec![
      rating_count_rank;
      self.settings.rating_count_weight as usize
    ]);
    ranks.append(&mut vec![
      descriptor_count_rank;
      self.settings.descriptor_count_weight as usize
    ]);
    ranks.append(&mut vec![
      average_credit_tag_rank;
      self.settings.credit_tag_weight as usize
    ]);

    let mut metadata = HashMap::new();
    metadata.insert(
      "average_primary_genre_rank".to_string(),
      average_primary_genre_rank.to_string(),
    );
    metadata.insert(
      "average_secondary_genre_rank".to_string(),
      average_secondary_genre_rank.to_string(),
    );
    metadata.insert(
      "average_descriptor_rank".to_string(),
      average_descriptor_rank.to_string(),
    );
    metadata.insert(
      "average_credit_tag_rank".to_string(),
      average_credit_tag_rank.to_string(),
    );
    metadata.insert("rating_rank".to_string(), rating_rank.to_string());
    metadata.insert(
      "rating_count_rank".to_string(),
      rating_count_rank.to_string(),
    );
    metadata.insert(
      "descriptor_count_rank".to_string(),
      descriptor_count_rank.to_string(),
    );

    let score = ranks.iter().sum::<f64>() / ranks.len() as f64;

    if score.is_nan() {
      Err(anyhow!("score is NaN"))
    } else {
      Ok(AlbumAssessment {
        score: score as f32,
        metadata: Some(metadata),
      })
    }
  }
}
