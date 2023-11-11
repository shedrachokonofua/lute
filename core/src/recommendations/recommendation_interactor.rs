use super::{
  embedding_similarity::embedding_similarity_interactor::{
    EmbeddingSimilarityAlbumAssessmentSettings, EmbeddingSimilarityAssessableAlbum,
    EmbeddingSimilarityInteractor,
  },
  quantile_ranking::quantile_rank_interactor::{
    QuantileRankAlbumAssessmentSettings, QuantileRankAssessableAlbum, QuantileRankInteractor,
  },
  types::{
    AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings,
    RecommendationMethodInteractor,
  },
};
use crate::{
  albums::{album_repository::AlbumRepository, album_search_index::AlbumSearchIndex},
  files::file_metadata::file_name::FileName,
  profile::{profile::ProfileId, profile_interactor::ProfileInteractor},
  settings::Settings,
  sqlite::SqliteConnection,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub enum AlbumAssessmentSettings {
  QuantileRank(QuantileRankAlbumAssessmentSettings),
  EmbeddingSimilarity(EmbeddingSimilarityAlbumAssessmentSettings),
}

pub struct RecommendationInteractor {
  quantile_rank_interactor: QuantileRankInteractor,
  embedding_similarity_interactor: EmbeddingSimilarityInteractor,
  album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  profile_interactor: ProfileInteractor,
}

impl RecommendationInteractor {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    sqlite_connection: Arc<SqliteConnection>,
    album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
    album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
  ) -> Self {
    Self {
      quantile_rank_interactor: QuantileRankInteractor::new(Arc::clone(&album_search_index)),
      embedding_similarity_interactor: EmbeddingSimilarityInteractor::new(Arc::clone(
        &album_search_index,
      )),
      album_repository: Arc::clone(&album_repository),
      profile_interactor: ProfileInteractor::new(
        settings,
        redis_connection_pool,
        sqlite_connection,
        Arc::clone(&album_repository),
      ),
    }
  }

  pub async fn assess_album(
    &self,
    profile_id: &ProfileId,
    album_file_name: &FileName,
    settings: AlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    let profile = self.profile_interactor.get_profile(profile_id).await?;
    let albums = self
      .album_repository
      .get_many(profile.album_file_names())
      .await?;
    let album = self.album_repository.get(album_file_name).await?;
    match settings {
      AlbumAssessmentSettings::QuantileRank(settings) => {
        self
          .quantile_rank_interactor
          .assess_album(
            &profile,
            &albums,
            &QuantileRankAssessableAlbum::try_from(album)?,
            settings,
          )
          .await
      }
      AlbumAssessmentSettings::EmbeddingSimilarity(settings) => {
        self
          .embedding_similarity_interactor
          .assess_album(
            &profile,
            &albums,
            &EmbeddingSimilarityAssessableAlbum::try_from(album)?,
            settings,
          )
          .await
      }
    }
  }

  pub async fn recommend_albums(
    &self,
    profile_id: &ProfileId,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let profile = self.profile_interactor.get_profile(profile_id).await?;
    let albums = self
      .album_repository
      .get_many(profile.album_file_names())
      .await?;

    match assessment_settings {
      AlbumAssessmentSettings::QuantileRank(settings) => {
        self
          .quantile_rank_interactor
          .recommend_albums(&profile, &albums, settings, recommendation_settings)
          .await
      }
      AlbumAssessmentSettings::EmbeddingSimilarity(settings) => {
        self
          .embedding_similarity_interactor
          .recommend_albums(&profile, &albums, settings, recommendation_settings)
          .await
      }
    }
  }
}
