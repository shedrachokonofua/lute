use super::{
  embedding_similarity::embedding_similarity_interactor::{
    EmbeddingSimilarityAlbumAssessmentSettings, EmbeddingSimilarityAssessableAlbum,
    EmbeddingSimilarityInteractor,
  },
  quantile_ranking::quantile_rank_interactor::{
    QuantileRankAlbumAssessmentSettings, QuantileRankAssessableAlbum, QuantileRankInteractor,
  },
  reranked_embedding_similarity::reranked_embedding_similarity_interactor::{
    RerankedEmbeddingSimilarityAlbumAssessmentSettings, RerankedEmbeddingSimilarityAssessableAlbum,
    RerankedEmbeddingSimilarityInteractor,
  },
  seed::{AlbumRecommendationSeed, AlbumRecommendationSeedContext},
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
  albums::{album_interactor::AlbumInteractor, album_read_model::AlbumReadModel},
  context::ApplicationContext,
  files::file_metadata::file_name::FileName,
  helpers::{embedding::average_embedding, redisearch::SearchPagination},
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
  RerankedEmbeddingSimilarity(RerankedEmbeddingSimilarityAlbumAssessmentSettings),
}

pub struct RecommendationInteractor {
  quantile_rank_interactor: Arc<QuantileRankInteractor>,
  embedding_similarity_interactor: Arc<EmbeddingSimilarityInteractor>,
  reranked_embedding_similarity_interactor: RerankedEmbeddingSimilarityInteractor,
  album_interactor: Arc<AlbumInteractor>,
  profile_interactor: Arc<ProfileInteractor>,
  spotify_track_search_index: Arc<SpotifyTrackSearchIndex>,
  spotify_client: Arc<SpotifyClient>,
}

impl RecommendationInteractor {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    let quantile_rank_interactor = Arc::new(QuantileRankInteractor::new(Arc::clone(
      &app_context.album_interactor,
    )));
    let embedding_similarity_interactor = Arc::new(EmbeddingSimilarityInteractor::new(Arc::clone(
      &app_context.album_interactor,
    )));
    let reranked_embedding_similarity_interactor = RerankedEmbeddingSimilarityInteractor::new(
      Arc::clone(&embedding_similarity_interactor),
      Arc::clone(&quantile_rank_interactor),
    );
    Self {
      quantile_rank_interactor,
      embedding_similarity_interactor,
      reranked_embedding_similarity_interactor,
      album_interactor: Arc::clone(&app_context.album_interactor),
      profile_interactor: Arc::clone(&app_context.profile_interactor),
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
      .album_interactor
      .find_many(profile.album_file_names())
      .await?
      .into_values()
      .collect();
    Ok((profile, albums))
  }

  async fn build_seed_context(
    &self,
    seed: AlbumRecommendationSeed,
  ) -> Result<AlbumRecommendationSeedContext> {
    match seed {
      AlbumRecommendationSeed::Profile(profile_id) => {
        let (profile, albums) = self.get_profile_and_albums(&profile_id).await?;
        Ok(AlbumRecommendationSeedContext::new(
          albums,
          profile.albums.clone(),
        ))
      }
      AlbumRecommendationSeed::Albums(factor_map) => {
        let albums = self
          .album_interactor
          .find_many(factor_map.keys().cloned().collect())
          .await?
          .into_values()
          .collect();
        Ok(AlbumRecommendationSeedContext::new(albums, factor_map))
      }
    }
  }

  pub async fn assess_album(
    &self,
    seed: AlbumRecommendationSeed,
    album_file_name: &FileName,
    settings: AlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    let seed_context = self.build_seed_context(seed).await?;
    let album = self.album_interactor.get(album_file_name).await?;
    match settings {
      AlbumAssessmentSettings::QuantileRank(settings) => {
        self
          .quantile_rank_interactor
          .assess_album(
            &seed_context,
            &QuantileRankAssessableAlbum::try_from(album)?,
            settings,
          )
          .await
      }
      AlbumAssessmentSettings::EmbeddingSimilarity(settings) => {
        self
          .embedding_similarity_interactor
          .assess_album(
            &seed_context,
            &EmbeddingSimilarityAssessableAlbum::try_from(album)?,
            settings,
          )
          .await
      }
      AlbumAssessmentSettings::RerankedEmbeddingSimilarity(settings) => {
        self
          .reranked_embedding_similarity_interactor
          .assess_album(
            &seed_context,
            &RerankedEmbeddingSimilarityAssessableAlbum::try_from(album)?,
            settings,
          )
          .await
      }
    }
  }

  async fn recommend_albums_with_seed_context(
    &self,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
    seed_context: &AlbumRecommendationSeedContext,
  ) -> Result<Vec<AlbumRecommendation>> {
    match assessment_settings {
      AlbumAssessmentSettings::QuantileRank(settings) => {
        self
          .quantile_rank_interactor
          .recommend_albums(seed_context, settings, recommendation_settings)
          .await
      }
      AlbumAssessmentSettings::EmbeddingSimilarity(settings) => {
        self
          .embedding_similarity_interactor
          .recommend_albums(seed_context, settings, recommendation_settings)
          .await
      }
      AlbumAssessmentSettings::RerankedEmbeddingSimilarity(settings) => {
        self
          .reranked_embedding_similarity_interactor
          .recommend_albums(seed_context, settings, recommendation_settings)
          .await
      }
    }
  }

  pub async fn recommend_albums(
    &self,
    seed: AlbumRecommendationSeed,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let seed_context = self.build_seed_context(seed).await?;
    self
      .recommend_albums_with_seed_context(
        assessment_settings,
        recommendation_settings,
        &seed_context,
      )
      .await
  }

  pub async fn draft_spotify_playlist(
    &self,
    seed: AlbumRecommendationSeed,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<SpotifyTrackReference>> {
    let seed_context = self.build_seed_context(seed).await?;
    let profile_tracks = self
      .spotify_track_search_index
      .search(
        &SpotifyTrackQueryBuilder::default()
          .include_album_file_names(seed_context.album_file_names())
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
            seed_context.get_factor(&track.album_file_name).unwrap_or(1),
          )
        })
        .collect::<Vec<_>>(),
    );
    let recommendations = self
      .recommend_albums_with_seed_context(
        assessment_settings,
        recommendation_settings,
        &seed_context,
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
    seed: AlbumRecommendationSeed,
    assessment_settings: AlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
    name: String,
    description: Option<String>,
  ) -> Result<(String, Vec<SpotifyTrackReference>)> {
    let playlist_draft = self
      .draft_spotify_playlist(seed, assessment_settings, recommendation_settings)
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
