use super::types::{
  AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings, RecommendationMethodInteractor,
};
use crate::{
  albums::album_read_model_repository::{
    AlbumReadModel, AlbumReadModelRepository, AlbumSearchQueryBuilder,
  },
  helpers::{
    bounded_min_heap::BoundedMinHeap, math::default_if_zero, quantile_rank::QuantileRanking,
  },
  profile::{profile::Profile, profile_summary::ItemWithFactor},
};
use anyhow::Result;
use async_trait::async_trait;
use derive_builder::Builder;
use num_traits::Zero;
use ordered_float::OrderedFloat;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tracing::{instrument, warn};

#[derive(Builder, Clone, Debug)]
#[builder(setter(into), default)]
pub struct QuantileRankAlbumAssessmentSettings {
  pub primary_genre_weight: u32,
  pub secondary_genre_weight: u32,
  pub descriptor_weight: u32,
  pub rating_weight: u32,
  pub rating_count_weight: u32,
  pub novelty_score: f64,
  pub descriptor_count_weight: u32,
  pub credit_tag_weight: u32,
}

impl Default for QuantileRankAlbumAssessmentSettings {
  fn default() -> Self {
    Self {
      primary_genre_weight: 4,
      secondary_genre_weight: 2,
      descriptor_weight: 7,
      rating_weight: 2,
      rating_count_weight: 1,
      novelty_score: 0.5,
      descriptor_count_weight: 2,
      credit_tag_weight: 1,
    }
  }
}

pub struct QuantileRankInteractor {
  album_read_model_repository: AlbumReadModelRepository,
}

impl QuantileRankInteractor {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      album_read_model_repository: AlbumReadModelRepository::new(Arc::clone(
        &redis_connection_pool,
      )),
    }
  }
}

#[derive(Clone, Debug)]
pub struct QuantileRankAssessableAlbum(AlbumReadModel);

impl TryFrom<AlbumReadModel> for QuantileRankAssessableAlbum {
  type Error = anyhow::Error;

  fn try_from(album_read_model: AlbumReadModel) -> Result<Self, Self::Error> {
    if album_read_model.descriptors.len() < 5 {
      return Err(anyhow::anyhow!("Not enough descriptors"));
    }

    Ok(Self(album_read_model))
  }
}

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

#[async_trait]
impl
  RecommendationMethodInteractor<QuantileRankAssessableAlbum, QuantileRankAlbumAssessmentSettings>
  for QuantileRankInteractor
{
  #[instrument(name = "QuantileRankInteractor::assess_album", skip(self))]
  async fn assess_album(
    &self,
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
    album_read_model: &QuantileRankAssessableAlbum,
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    let profile_summary = profile.summarize(profile_albums);
    let average_primary_genre_rank = calculate_average_rank(
      &QuantileRanking::new(&profile_summary.primary_genres),
      &create_item_with_factor_map(&profile_summary.primary_genres),
      &album_read_model.0.primary_genres,
      settings.novelty_score,
    )?;
    let average_secondary_genre_rank = calculate_average_rank(
      &QuantileRanking::new(&profile_summary.secondary_genres),
      &create_item_with_factor_map(&profile_summary.secondary_genres),
      &album_read_model.0.secondary_genres,
      settings.novelty_score,
    )?;
    let average_descriptor_rank = calculate_average_rank(
      &QuantileRanking::new(&profile_summary.descriptors),
      &create_item_with_factor_map(&profile_summary.descriptors),
      &album_read_model.0.descriptors,
      settings.novelty_score,
    )?;
    let average_credit_tag_rank = calculate_average_rank(
      &QuantileRanking::new(&profile_summary.credit_tags),
      &create_item_with_factor_map(&profile_summary.credit_tags),
      &album_read_model.0.credit_tags,
      0.1,
    )?;
    let rating_rank = default_if_zero(
      QuantileRanking::new(
        &profile_albums
          .iter()
          .map(|album| OrderedFloat(album.rating))
          .collect::<Vec<_>>(),
      )
      .get_rank(&OrderedFloat(album_read_model.0.rating)),
      settings.novelty_score,
    );
    let rating_count_rank = default_if_zero(
      QuantileRanking::new(
        &profile_albums
          .iter()
          .map(|album| album.rating_count)
          .collect::<Vec<_>>(),
      )
      .get_rank(&album_read_model.0.rating_count),
      settings.novelty_score,
    );
    let descriptor_count_rank = default_if_zero(
      QuantileRanking::new(
        &profile_albums
          .iter()
          .map(|album| album.descriptor_count)
          .collect::<Vec<_>>(),
      )
      .get_rank(&album_read_model.0.descriptor_count),
      settings.novelty_score,
    );
    let credit_tag_rank = default_if_zero(
      QuantileRanking::new(
        &profile_albums
          .iter()
          .map(|album| album.credit_tag_count)
          .collect::<Vec<_>>(),
      )
      .get_rank(&album_read_model.0.credit_tag_count),
      0.1,
    );

    let mut ranks = vec![average_primary_genre_rank; settings.primary_genre_weight as usize];
    ranks.append(&mut vec![
      average_secondary_genre_rank;
      settings.secondary_genre_weight as usize
    ]);
    ranks.append(&mut vec![
      average_descriptor_rank;
      settings.descriptor_weight as usize
    ]);
    ranks.append(&mut vec![rating_rank; settings.rating_weight as usize]);
    ranks.append(&mut vec![
      rating_count_rank;
      settings.rating_count_weight as usize
    ]);
    ranks.append(&mut vec![
      descriptor_count_rank;
      settings.descriptor_count_weight as usize
    ]);
    ranks.append(&mut vec![
      credit_tag_rank;
      settings.credit_tag_weight as usize
    ]);
    let score = ranks.iter().sum::<f64>() / ranks.len() as f64;

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

    Ok(AlbumAssessment {
      score: score as f32,
      metadata: Some(metadata),
    })
  }

  #[instrument(name = "QuantileRankInteractor::recommend_albums", skip(self))]
  async fn recommend_albums(
    &self,
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
    assessment_settings: QuantileRankAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let profile_summary = profile.summarize(profile_albums);
    let mut search_query_builder = AlbumSearchQueryBuilder::default();
    search_query_builder
      .exclude_file_names(profile.albums.keys().cloned().collect::<Vec<_>>())
      .include_primary_genres(recommendation_settings.include_primary_genres)
      .include_secondary_genres(recommendation_settings.include_secondary_genres)
      .include_languages(recommendation_settings.include_languages)
      .exclude_primary_genres(recommendation_settings.exclude_primary_genres)
      .exclude_secondary_genres(recommendation_settings.exclude_secondary_genres)
      .exclude_languages(recommendation_settings.exclude_languages)
      .min_release_year(recommendation_settings.min_release_year)
      .max_release_year(recommendation_settings.max_release_year)
      .min_primary_genre_count(1)
      .min_secondary_genre_count(1)
      .min_descriptor_count(5);

    if recommendation_settings
      .exclude_known_artists
      .unwrap_or(false)
    {
      search_query_builder.exclude_artists(
        profile_albums
          .iter()
          .flat_map(|album| album.artists.clone())
          .map(|artist| artist.file_name.to_string())
          .collect::<Vec<_>>(),
      );
    }

    let search_query = search_query_builder.build()?;
    let albums = self
      .album_read_model_repository
      .search(&search_query)
      .await?;

    let primary_genre_ranking = QuantileRanking::new(&profile_summary.primary_genres);
    let secondary_genre_ranking = QuantileRanking::new(&profile_summary.secondary_genres);
    let descriptor_ranking = QuantileRanking::new(&profile_summary.descriptors);
    let rating_ranking = QuantileRanking::new(
      &profile_albums
        .iter()
        .map(|album| OrderedFloat(album.rating))
        .collect::<Vec<_>>(),
    );
    let rating_count_ranking = QuantileRanking::new(
      &profile_albums
        .iter()
        .map(|album| album.rating_count)
        .collect::<Vec<_>>(),
    );
    let descriptor_count_ranking = QuantileRanking::new(
      &profile_albums
        .iter()
        .map(|album| album.descriptor_count)
        .collect::<Vec<_>>(),
    );
    let credit_tag_ranking = QuantileRanking::new(&profile_summary.credit_tags);
    let result_heap = Arc::new(Mutex::new(BoundedMinHeap::new(
      recommendation_settings.count as usize,
    )));

    let primary_genre_summary_map = create_item_with_factor_map(&profile_summary.primary_genres);
    let secondary_genre_summary_map =
      create_item_with_factor_map(&profile_summary.secondary_genres);
    let descriptor_summary_map = create_item_with_factor_map(&profile_summary.descriptors);
    let credit_tag_summary_map = create_item_with_factor_map(&profile_summary.credit_tags);

    let (recommendation_sender, mut recommendation_receiver) = mpsc::unbounded_channel();
    rayon::spawn(move || {
      albums.par_iter().for_each(|album| {
        let average_primary_genre_rank = if !assessment_settings.primary_genre_weight.is_zero() {
          calculate_average_rank(
            &primary_genre_ranking,
            &primary_genre_summary_map,
            &album.primary_genres,
            assessment_settings.novelty_score,
          )
          .unwrap()
        } else {
          0.0
        };

        let average_secondary_genre_rank = if !assessment_settings.secondary_genre_weight.is_zero()
        {
          calculate_average_rank(
            &secondary_genre_ranking,
            &secondary_genre_summary_map,
            &album.secondary_genres,
            assessment_settings.novelty_score,
          )
          .unwrap()
        } else {
          0.0
        };

        let average_descriptor_rank = if !assessment_settings.descriptor_weight.is_zero() {
          calculate_average_rank(
            &descriptor_ranking,
            &descriptor_summary_map,
            &album.descriptors,
            assessment_settings.novelty_score,
          )
          .unwrap()
        } else {
          0.0
        };

        let average_credit_tag_rank = if !assessment_settings.credit_tag_weight.is_zero() {
          calculate_average_rank(
            &credit_tag_ranking,
            &credit_tag_summary_map,
            &album.credit_tags,
            0.1,
          )
          .unwrap()
        } else {
          0.0
        };

        let rating_rank = if !assessment_settings.rating_weight.is_zero() {
          default_if_zero(
            rating_ranking.get_rank(&OrderedFloat(album.rating)),
            assessment_settings.novelty_score,
          )
        } else {
          0.0
        };

        let rating_count_rank = if !assessment_settings.rating_count_weight.is_zero() {
          default_if_zero(
            rating_count_ranking.get_rank(&album.rating_count),
            assessment_settings.novelty_score,
          )
        } else {
          0.0
        };

        let descriptor_count_rank = if !assessment_settings.descriptor_count_weight.is_zero() {
          default_if_zero(
            descriptor_count_ranking.get_rank(&album.descriptor_count),
            assessment_settings.novelty_score,
          )
        } else {
          0.0
        };

        let mut ranks =
          vec![average_primary_genre_rank; assessment_settings.primary_genre_weight as usize];
        ranks.append(&mut vec![
          average_secondary_genre_rank;
          assessment_settings.secondary_genre_weight as usize
        ]);
        ranks.append(&mut vec![
          average_descriptor_rank;
          assessment_settings.descriptor_weight as usize
        ]);
        ranks.append(&mut vec![
          rating_rank;
          assessment_settings.rating_weight as usize
        ]);
        ranks.append(&mut vec![
          rating_count_rank;
          assessment_settings.rating_count_weight as usize
        ]);
        ranks.append(&mut vec![
          descriptor_count_rank;
          assessment_settings.descriptor_count_weight
            as usize
        ]);
        ranks.append(&mut vec![
          average_credit_tag_rank;
          assessment_settings.credit_tag_weight as usize
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
          warn!(
            "score is NaN, {}, {}",
            average_credit_tag_rank,
            album.file_name.to_string()
          );
        } else {
          let recommendation = AlbumRecommendation {
            album: album.clone(),
            assessment: AlbumAssessment {
              score: score as f32,
              metadata: Some(metadata),
            },
          };

          recommendation_sender.send(recommendation).unwrap();
        }
      });
    });
    while let Some(recommendation) = recommendation_receiver.recv().await {
      result_heap.lock().unwrap().push(recommendation);
    }
    let recommendations = (*result_heap.lock().unwrap()).drain_sorted_desc();
    Ok(recommendations)
  }
}
