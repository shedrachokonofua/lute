use super::{
  quantile_rank_interactor::{
    QuantileRankAlbumAssessmentSettings, QuantileRankAssessableAlbum, QuantileRankInteractor,
  },
  types::{AlbumAssessment, RecommendationMethodInteractor},
};
use crate::{
  albums::album_read_model_repository::AlbumReadModelRepository,
  files::file_metadata::file_name::FileName,
  profile::{profile::ProfileId, profile_interactor::ProfileInteractor},
  settings::Settings,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub enum AlbumAssessmentSettings {
  QuantileRank(QuantileRankAlbumAssessmentSettings),
}

pub struct RecommendationInteractor {
  quantile_rank_interactor: QuantileRankInteractor,
  album_read_model_repository: AlbumReadModelRepository,
  profile_interactor: ProfileInteractor,
}

impl RecommendationInteractor {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
  ) -> Self {
    Self {
      quantile_rank_interactor: QuantileRankInteractor::new(Arc::clone(&redis_connection_pool)),
      album_read_model_repository: AlbumReadModelRepository::new(Arc::clone(
        &redis_connection_pool,
      )),
      profile_interactor: ProfileInteractor::new(settings, redis_connection_pool),
    }
  }

  pub async fn assess_album(
    &self,
    profile_id: &ProfileId,
    album_file_name: &FileName,
    settings: AlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    let profile_summary = self
      .profile_interactor
      .get_profile_summary(profile_id)
      .await?;
    let album = self
      .album_read_model_repository
      .get(album_file_name)
      .await?;
    match settings {
      AlbumAssessmentSettings::QuantileRank(settings) => {
        self
          .quantile_rank_interactor
          .assess_album(
            &profile_summary,
            &QuantileRankAssessableAlbum::try_from(album)?,
            settings,
          )
          .await
      }
    }
  }
}
