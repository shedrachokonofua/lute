use super::types::{AlbumAssessment, RecommendationMethodInteractor};
use crate::{
  albums::album_read_model_repository::{AlbumReadModel, AlbumReadModelRepository},
  profile::profile_summary::ProfileSummary,
};
use anyhow::Result;
use async_trait::async_trait;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone, Debug)]
pub struct QuantileRankAlbumAssessmentSettings {
  pub primary_genre_weight: f32,
  pub secondary_genre_weight: f32,
  pub descriptor_weight: f32,
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

#[async_trait]
impl RecommendationMethodInteractor<QuantileRankAlbumAssessmentSettings>
  for QuantileRankInteractor
{
  #[instrument(name = "QuantileRankInteractor::assess_album", skip(self))]
  async fn assess_album(
    &self,
    profile_summary: &ProfileSummary,
    album_read_model: &AlbumReadModel,
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    Err(anyhow::anyhow!("Not implemented"))
  }
}
