use super::types::{
  AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings, RecommendationMethodInteractor,
};
use crate::{
  albums::album_read_model_repository::{
    AlbumReadModel, AlbumReadModelRepository, AlbumSearchQueryBuilder,
  },
  helpers::{bounded_min_heap::BoundedMinHeap, quantile_rank::QuantileRanking},
  profile::{
    profile::Profile,
    profile_summary::{ItemWithFactor, ProfileSummary},
  },
};
use anyhow::Result;
use async_trait::async_trait;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{instrument, warn};

#[derive(Clone, Debug)]
pub struct QuantileRankAlbumAssessmentSettings {
  pub primary_genre_weight: u32,
  pub secondary_genre_weight: u32,
  pub descriptor_weight: u32,
  pub novelty_score: f64,
}

impl Default for QuantileRankAlbumAssessmentSettings {
  fn default() -> Self {
    Self {
      primary_genre_weight: 6,
      secondary_genre_weight: 3,
      descriptor_weight: 20,
      novelty_score: 0.5,
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

fn calculate_average_rank(
  ranking: &QuantileRanking<ItemWithFactor>,
  profile_tags: &Vec<ItemWithFactor>,
  album_tags: &Vec<String>,
  novelty_score: f64,
) -> Result<f64> {
  let ranks = album_tags
    .iter()
    .map(|tag: &String| {
      let item = profile_tags.iter().find(|item| &item.item == tag);
      let rank = match item {
        Some(item) => ranking.get_rank(item),
        None => {
          warn!("Tag {} not found in profile", tag);
          None
        }
      };
      rank.unwrap_or(novelty_score)
    })
    .collect::<Vec<f64>>();

  Ok(ranks.iter().sum::<f64>() / ranks.len() as f64)
}

#[async_trait]
impl
  RecommendationMethodInteractor<QuantileRankAssessableAlbum, QuantileRankAlbumAssessmentSettings>
  for QuantileRankInteractor
{
  #[instrument(name = "QuantileRankInteractor::assess_album", skip(self))]
  async fn assess_album(
    &self,
    profile_summary: &ProfileSummary,
    album_read_model: &QuantileRankAssessableAlbum,
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    let average_primary_genre_rank = calculate_average_rank(
      &QuantileRanking::new(&profile_summary.primary_genres),
      &profile_summary.primary_genres,
      &album_read_model.0.primary_genres,
      settings.novelty_score,
    )?;
    let average_secondary_genre_rank = calculate_average_rank(
      &QuantileRanking::new(&profile_summary.secondary_genres),
      &profile_summary.secondary_genres,
      &album_read_model.0.secondary_genres,
      settings.novelty_score,
    )?;
    let average_descriptor_rank = calculate_average_rank(
      &QuantileRanking::new(&profile_summary.descriptors),
      &profile_summary.descriptors,
      &album_read_model.0.descriptors,
      settings.novelty_score,
    )?;

    let mut ranks = vec![average_primary_genre_rank; settings.primary_genre_weight as usize];
    ranks.append(&mut vec![
      average_secondary_genre_rank;
      settings.secondary_genre_weight as usize
    ]);
    ranks.append(&mut vec![
      average_descriptor_rank;
      settings.descriptor_weight as usize
    ]);
    let score = ranks.iter().sum::<f64>() / ranks.len() as f64;

    Ok(AlbumAssessment {
      score: score as f32,
      metadata: None,
    })
  }

  async fn recommend_albums(
    &self,
    profile: &Profile,
    profile_summary: ProfileSummary,
    assessment_settings: QuantileRankAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let search_query = AlbumSearchQueryBuilder::default()
      .exclude_file_names(profile.albums.keys().cloned().collect::<Vec<_>>())
      .min_primary_genre_count(1)
      .min_secondary_genre_count(1)
      .min_descriptor_count(5)
      .build()?;
    let albums = self
      .album_read_model_repository
      .search(&search_query, Some(0), Some(10000))
      .await?;

    let primary_genre_ranking = QuantileRanking::new(&profile_summary.primary_genres);
    let secondary_genre_ranking = QuantileRanking::new(&profile_summary.secondary_genres);
    let descriptor_ranking = QuantileRanking::new(&profile_summary.descriptors);
    let result_heap = Arc::new(Mutex::new(BoundedMinHeap::new(
      recommendation_settings.count as usize,
    )));

    let (recommendation_sender, mut recommendation_receiver) = mpsc::unbounded_channel();
    rayon::spawn(move || {
      albums.par_iter().for_each(|album| {
        let average_primary_genre_rank = calculate_average_rank(
          &primary_genre_ranking,
          &profile_summary.primary_genres,
          &album.primary_genres,
          assessment_settings.novelty_score,
        )
        .unwrap();

        let average_secondary_genre_rank = calculate_average_rank(
          &secondary_genre_ranking,
          &profile_summary.secondary_genres,
          &album.secondary_genres,
          assessment_settings.novelty_score,
        )
        .unwrap();
        let average_descriptor_rank = calculate_average_rank(
          &descriptor_ranking,
          &profile_summary.descriptors,
          &album.descriptors,
          assessment_settings.novelty_score,
        )
        .unwrap();

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
        let score = ranks.iter().sum::<f64>() / ranks.len() as f64;
        if score.is_nan() {
          warn!("score is NaN");
        } else {
          let recommendation = AlbumRecommendation {
            album: album.clone(),
            assessment: AlbumAssessment {
              score: score as f32,
              metadata: None,
            },
          };

          recommendation_sender.send(recommendation).unwrap();
        }
      });
    });
    while let Some(recommendation) = recommendation_receiver.recv().await {
      result_heap.lock().unwrap().push(recommendation);
    }
    let mut recommendations = (*result_heap.lock().unwrap()).drain();
    recommendations.sort();
    Ok(recommendations)
  }
}
