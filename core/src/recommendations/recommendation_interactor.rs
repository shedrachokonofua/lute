use super::{
  quantile_rank_interactor::{QuantileRankAlbumAssessmentSettings, QuantileRankInteractor},
  types::{AlbumAssessment, RecommendationMethodInteractor},
};
use crate::{files::file_metadata::file_name::FileName, profile::profile::ProfileId};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub enum AlbumAssessmentSettings {
  QuantileRank(QuantileRankAlbumAssessmentSettings),
}

pub struct RecommendationInteractor {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  quantile_rank_interactor: QuantileRankInteractor,
}

impl RecommendationInteractor {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      quantile_rank_interactor: QuantileRankInteractor::new(redis_connection_pool),
    }
  }

  pub fn assess_album(
    &self,
    profile_id: &ProfileId,
    album_file_name: &FileName,
    settings: AlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    match settings {
      AlbumAssessmentSettings::QuantileRank(settings) => self
        .quantile_rank_interactor
        .assess_album(profile_id, album_file_name, settings),
    }
  }
}
