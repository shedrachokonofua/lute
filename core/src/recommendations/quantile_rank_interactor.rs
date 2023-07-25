use super::types::{AlbumAssessment, RecommendationMethodInteractor};
use crate::{
  albums::album_read_model_repository::{AlbumReadModel, AlbumReadModelRepository},
  helpers::quantile_rank::QuantileRanking,
  profile::profile_summary::ProfileSummary,
};
use anyhow::Result;
use async_trait::async_trait;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone, Debug)]
pub struct QuantileRankAlbumAssessmentSettings {
  pub primary_genre_weight: u32,
  pub secondary_genre_weight: u32,
  pub descriptor_weight: u32,
}

impl Default for QuantileRankAlbumAssessmentSettings {
  fn default() -> Self {
    Self {
      primary_genre_weight: 6,
      secondary_genre_weight: 3,
      descriptor_weight: 20,
    }
  }
}

pub struct QuantileRankInteractor {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  album_read_model_repository: AlbumReadModelRepository,
}

impl QuantileRankInteractor {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
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
    let primary_genre_ranker = QuantileRanking::new(profile_summary.primary_genres.clone());
    let primary_genre_ranks = album_read_model
      .0
      .primary_genres
      .iter()
      .map(|genre| {
        primary_genre_ranker.get_rank(
          profile_summary
            .primary_genres
            .iter()
            .find(|g| &g.item == genre)
            .unwrap()
            .clone(),
        )
      })
      .collect::<Option<Vec<f64>>>()
      .unwrap_or_default();
    let average_primary_genre_rank =
      primary_genre_ranks.iter().sum::<f64>() / primary_genre_ranks.len() as f64;

    let secondary_genre_ranker = QuantileRanking::new(profile_summary.secondary_genres.clone());
    let secondary_genre_ranks = album_read_model
      .0
      .secondary_genres
      .iter()
      .map(|genre| {
        secondary_genre_ranker.get_rank(
          profile_summary
            .secondary_genres
            .iter()
            .find(|g| &g.item == genre)
            .unwrap()
            .clone(),
        )
      })
      .collect::<Option<Vec<f64>>>()
      .unwrap_or_default();
    let average_secondary_genre_rank =
      secondary_genre_ranks.iter().sum::<f64>() / secondary_genre_ranks.len() as f64;

    let descriptor_ranker = QuantileRanking::new(profile_summary.descriptors.clone());
    let descriptor_ranks = album_read_model
      .0
      .descriptors
      .iter()
      .map(|descriptor| {
        descriptor_ranker.get_rank(
          profile_summary
            .descriptors
            .iter()
            .find(|d| &d.item == descriptor)
            .unwrap()
            .clone(),
        )
      })
      .collect::<Option<Vec<f64>>>()
      .unwrap_or_default();
    let average_descriptor_rank =
      descriptor_ranks.iter().sum::<f64>() / descriptor_ranks.len() as f64;

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
}
