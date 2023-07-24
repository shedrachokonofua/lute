use super::types::{AlbumAssessment, RecommendationMethodInteractor};
use crate::{
  albums::album_read_model_repository::AlbumReadModelRepository,
  files::file_metadata::file_name::FileName, profile::profile::ProfileId,
};
use anyhow::Result;
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
  album_read_model_repository: Arc<AlbumReadModelRepository>,
}

impl QuantileRankInteractor {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      album_read_model_repository: Arc::new(AlbumReadModelRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      }),
    }
  }
}

impl RecommendationMethodInteractor<QuantileRankAlbumAssessmentSettings>
  for QuantileRankInteractor
{
  #[instrument(name = "QuantileRankInteractor::assess_album", skip(self))]
  fn assess_album(
    &self,
    profile_id: &ProfileId,
    album_file_name: &FileName,
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    Err(anyhow::anyhow!("Not implemented"))
  }
}
