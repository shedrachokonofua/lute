use super::types::{
  AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings, RecommendationMethodInteractor,
};
use crate::{
  albums::album_read_model_repository::{AlbumReadModel, AlbumReadModelRepository},
  helpers::quantile_rank::QuantileRanking,
  profile::profile_summary::{ItemWithFactor, ProfileSummary},
};
use anyhow::Result;
use async_trait::async_trait;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
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

impl QuantileRankInteractor {
  fn calculate_average_rank(
    &self,
    profile_tags: &Vec<ItemWithFactor>,
    album_tags: &Vec<String>,
    novelty_score: f64,
  ) -> Result<f64> {
    let ranker: QuantileRanking<ItemWithFactor> = QuantileRanking::new(profile_tags.clone());
    let ranks = album_tags
      .iter()
      .map(|tag: &String| {
        let item = profile_tags.iter().find(|item| &item.item == tag);
        let rank = match item {
          Some(item) => ranker.get_rank(item),
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
    let average_primary_genre_rank = self.calculate_average_rank(
      &profile_summary.primary_genres,
      &album_read_model.0.primary_genres,
      settings.novelty_score,
    )?;
    let average_secondary_genre_rank = self.calculate_average_rank(
      &profile_summary.secondary_genres,
      &album_read_model.0.secondary_genres,
      settings.novelty_score,
    )?;
    let average_descriptor_rank = self.calculate_average_rank(
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
    profile_summary: &ProfileSummary,
    assessment_settings: QuantileRankAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    Err(anyhow::anyhow!("Not implemented"))
  }
}
