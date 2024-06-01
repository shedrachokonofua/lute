use super::{
  embedding_similarity::embedding_similarity_interactor::{
    EmbeddingSimilarityAlbumAssessmentSettings, EmbeddingSimilarityAssessableAlbum,
    EmbeddingSimilarityInteractor,
  },
  quantile_ranking::quantile_rank_interactor::{
    QuantileRankAlbumAssessmentSettings, QuantileRankAssessableAlbum, QuantileRankInteractor,
  },
  spotify_track_search_index::{
    SpotifyTrackEmbeddingSimilaritySearchQuery, SpotifyTrackQuery, SpotifyTrackQueryBuilder,
    SpotifyTrackSearchIndex, SpotifyTrackSearchResult,
  },
  types::{
    AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings,
    RecommendationMethodInteractor,
  },
};
use crate::{
  albums::{album_read_model::AlbumReadModel, album_repository::AlbumRepository},
  context::ApplicationContext,
  files::file_metadata::file_name::FileName,
  helpers::{math::average_embedding, redisearch::SearchPagination},
  profile::{
    profile::{Profile, ProfileId},
    profile_interactor::ProfileInteractor,
  },
  spotify::spotify_client::{SpotifyClient, SpotifyTrackReference},
};
use anyhow::Result;
use futures::future::join_all;
use std::sync::Arc;

pub enum AlbumAssessmentSettings {
  QuantileRank(QuantileRankAlbumAssessmentSettings),
  EmbeddingSimilarity(EmbeddingSimilarityAlbumAssessmentSettings),
}

pub struct RecommendationInteractor {
  quantile_rank_interactor: QuantileRankInteractor,
  embedding_similarity_interactor: EmbeddingSimilarityInteractor,
  album_repository: Arc<AlbumRepository>,
  profile_interactor: ProfileInteractor,
  spotify_track_search_index: Arc<SpotifyTrackSearchIndex>,
  spotify_client: Arc<SpotifyClient>,
}

impl RecommendationInteractor {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      quantile_rank_interactor: QuantileRankInteractor::new(Arc::clone(
        &app_context.album_search_index,
      )),
      embedding_similarity_interactor: EmbeddingSimilarityInteractor::new(Arc::clone(
        &app_context.album_search_index,
      )),
      album_repository: Arc::clone(&app_context.album_repository),
      profile_interactor: ProfileInteractor::new(
        Arc::clone(&app_context.settings),
        Arc::clone(&app_context.redis_connection_pool),
        Arc::clone(&app_context.sqlite_connection),
        Arc::clone(&app_context.album_repository),
        Arc::clone(&app_context.spotify_client),
      ),
      spotify_track_search_index: Arc::clone(&app_context.spotify_track_search_index),
      spotify_client: Arc::clone(&app_context.spotify_client),
    }
  }

  async fn get_profile_and_albums(
    &self,
    profile_id: &ProfileId,
  ) -> Result<(Profile, Vec<AlbumReadModel>)> {
    let profile = self.profile_interactor.get_profile(profile_id).await?;
    let albums = self
      .album_repository
      .find_many(profile.album_file_names())
      .await?;
    Ok((profile, albums))
  }

  pub async fn assess_album(
    &self,
    profile_id: &ProfileId,
    album_file_name: &FileName,
    settings: AlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    let (profile, albums) = self.get_profile_and_albums(profile_id).await?;
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

  async fn recommend_albums_with_profile(
    &self,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
    profile: &Profile,
    profile_albums: &Vec<AlbumReadModel>,
  ) -> Result<Vec<AlbumRecommendation>> {
    match assessment_settings {
      AlbumAssessmentSettings::QuantileRank(settings) => {
        self
          .quantile_rank_interactor
          .recommend_albums(profile, profile_albums, settings, recommendation_settings)
          .await
      }
      AlbumAssessmentSettings::EmbeddingSimilarity(settings) => {
        self
          .embedding_similarity_interactor
          .recommend_albums(profile, profile_albums, settings, recommendation_settings)
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
    let (profile, albums) = self.get_profile_and_albums(profile_id).await?;
    self
      .recommend_albums_with_profile(
        assessment_settings,
        recommendation_settings,
        &profile,
        &albums,
      )
      .await
  }

  pub async fn draft_spotify_playlist(
    &self,
    profile_id: &ProfileId,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<SpotifyTrackReference>> {
    let (profile, profile_albums) = self.get_profile_and_albums(profile_id).await?;
    let profile_tracks = self
      .spotify_track_search_index
      .search(
        &SpotifyTrackQueryBuilder::default()
          .include_album_file_names(profile.album_file_names())
          .build()?,
        None,
      )
      .await?;
    let profile_embedding = average_embedding(
      profile_tracks
        .tracks
        .iter()
        .map(|track| {
          (
            &track.embedding,
            profile
              .albums
              .get(&track.album_file_name)
              .copied()
              .unwrap_or(1),
          )
        })
        .collect::<Vec<_>>(),
    );
    let recommendations = self
      .recommend_albums_with_profile(
        assessment_settings,
        recommendation_settings,
        &profile,
        &profile_albums,
      )
      .await?;
    let recommendation_tracks = join_all(
      recommendations
        .iter()
        .map(|recommendation| async {
          let track = self
            .spotify_track_search_index
            .embedding_similarity_search(&SpotifyTrackEmbeddingSimilaritySearchQuery {
              embedding: profile_embedding.clone(),
              filters: SpotifyTrackQueryBuilder::default()
                .include_album_file_names(vec![recommendation.album.file_name.clone()])
                .build()?,
              limit: 1,
            })
            .await?;
          Ok(track.into_iter().next())
        })
        .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .filter_map(|result| result.map(|r| r.map(|(t, _)| t.into())).transpose())
    .collect::<Result<Vec<SpotifyTrackReference>>>()?;

    Ok(recommendation_tracks)
  }

  pub async fn create_spotify_playlist(
    &self,
    profile_id: &ProfileId,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
    name: String,
    description: Option<String>,
  ) -> Result<(String, Vec<SpotifyTrackReference>)> {
    let playlist_draft = self
      .draft_spotify_playlist(profile_id, assessment_settings, recommendation_settings)
      .await?;
    let playlist_id = self
      .spotify_client
      .create_playlist(
        name,
        description,
        playlist_draft
          .iter()
          .map(|t| t.spotify_id.clone())
          .collect(),
      )
      .await?;

    Ok((playlist_id, playlist_draft))
  }

  pub async fn search_spotify_track(
    &self,
    query: &SpotifyTrackQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<SpotifyTrackSearchResult> {
    self
      .spotify_track_search_index
      .search(query, pagination)
      .await
  }
}
