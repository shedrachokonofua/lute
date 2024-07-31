use super::{
  quantile_rank::QuantileRanking, quantile_rank_interactor::QuantileRankAlbumAssessmentSettings,
};
use crate::{
  albums::{album_collection_summary::AlbumCollectionSummary, album_read_model::AlbumReadModel},
  helpers::{item_with_factor::ItemWithFactor, math::default_if_zero},
  recommendations::{seed::AlbumRecommendationSeedContext, types::AlbumAssessment},
};
use anyhow::{anyhow, Result};
use num_traits::Zero;
use ordered_float::OrderedFloat;
use std::collections::HashMap;
use tracing::warn;

fn create_item_with_factor_map(items: Vec<ItemWithFactor>) -> HashMap<String, ItemWithFactor> {
  items
    .into_iter()
    .map(|item| (item.item.clone(), item))
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

fn compute_ranks<F>(weight: u32, compute_fn: F) -> Result<(f64, Vec<f64>)>
where
  F: FnOnce() -> Result<f64>,
{
  if weight.is_zero() {
    return Ok((0.0, vec![]));
  }

  match compute_fn() {
    Ok(rank) => Ok((rank, vec![rank; weight as usize])),
    Err(err) => Err(err),
  }
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
    seed_context: &AlbumRecommendationSeedContext,
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Self {
    let rating_ranking = QuantileRanking::new(
      &seed_context
        .albums
        .iter()
        .map(|album| OrderedFloat(album.rating))
        .collect::<Vec<_>>(),
    );
    let rating_count_ranking = QuantileRanking::new(
      &seed_context
        .albums
        .iter()
        .map(|album| album.rating_count)
        .collect::<Vec<_>>(),
    );
    let descriptor_count_ranking = QuantileRanking::new(
      &seed_context
        .albums
        .iter()
        .map(|album| album.descriptors.len() as u32)
        .collect::<Vec<_>>(),
    );
    let seed_summary = AlbumCollectionSummary::new(&seed_context.albums, &seed_context.factor_map);
    Self {
      settings,
      primary_genre_ranking: QuantileRanking::new(&seed_summary.primary_genres),
      secondary_genre_ranking: QuantileRanking::new(&seed_summary.secondary_genres),
      descriptor_ranking: QuantileRanking::new(&seed_summary.descriptors),
      credit_tag_ranking: QuantileRanking::new(&seed_summary.credit_tags),
      primary_genre_summary_map: create_item_with_factor_map(seed_summary.primary_genres),
      secondary_genre_summary_map: create_item_with_factor_map(seed_summary.secondary_genres),
      descriptor_summary_map: create_item_with_factor_map(seed_summary.descriptors),
      credit_tag_summary_map: create_item_with_factor_map(seed_summary.credit_tags),
      rating_ranking,
      rating_count_ranking,
      descriptor_count_ranking,
    }
  }

  pub fn assess(&self, album: &AlbumReadModel) -> Result<AlbumAssessment> {
    let (average_primary_genre_rank, mut primary_genre_ranks) =
      compute_ranks(self.settings.primary_genre_weight, || {
        calculate_average_rank(
          &self.primary_genre_ranking,
          &self.primary_genre_summary_map,
          &album.primary_genres,
          self.settings.novelty_score,
        )
      })?;
    let (average_secondary_genre_rank, mut secondary_genre_ranks) =
      compute_ranks(self.settings.secondary_genre_weight, || {
        calculate_average_rank(
          &self.secondary_genre_ranking,
          &self.secondary_genre_summary_map,
          &album.secondary_genres,
          self.settings.novelty_score,
        )
      })?;
    let (average_descriptor_rank, mut descriptor_ranks) =
      compute_ranks(self.settings.descriptor_weight, || {
        calculate_average_rank(
          &self.descriptor_ranking,
          &self.descriptor_summary_map,
          &album.descriptors,
          self.settings.novelty_score,
        )
      })?;
    let (average_credit_tag_rank, mut credit_tag_ranks) =
      compute_ranks(self.settings.credit_tag_weight, || {
        calculate_average_rank(
          &self.credit_tag_ranking,
          &self.credit_tag_summary_map,
          &album.credit_tags(),
          self.settings.novelty_score,
        )
      })?;
    let (rating_rank, mut rating_ranks) = compute_ranks(self.settings.rating_weight, || {
      Ok(self.rating_ranking.get_rank(&OrderedFloat(album.rating)))
    })?;
    let (rating_count_rank, mut rating_count_ranks) =
      compute_ranks(self.settings.rating_count_weight, || {
        Ok(self.rating_count_ranking.get_rank(&album.rating_count))
      })?;
    let (descriptor_count_rank, mut descriptor_count_ranks) =
      compute_ranks(self.settings.descriptor_count_weight, || {
        Ok(
          self
            .descriptor_count_ranking
            .get_rank(&(album.descriptors.len() as u32)),
        )
      })?;

    let mut ranks = vec![];
    ranks.append(&mut primary_genre_ranks);
    ranks.append(&mut secondary_genre_ranks);
    ranks.append(&mut descriptor_ranks);
    ranks.append(&mut credit_tag_ranks);
    ranks.append(&mut rating_ranks);
    ranks.append(&mut rating_count_ranks);
    ranks.append(&mut descriptor_count_ranks);

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
