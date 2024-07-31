use super::{
  embedding_similarity::embedding_similarity_interactor::EmbeddingSimilarityAlbumAssessmentSettings,
  quantile_ranking::quantile_rank_interactor::{
    QuantileRankAlbumAssessmentSettings, QuantileRankAlbumAssessmentSettingsBuilder,
  },
  recommendation_interactor::{AlbumAssessmentSettings, RecommendationInteractor},
  reranked_embedding_similarity::reranked_embedding_similarity_interactor::RerankedEmbeddingSimilarityAlbumAssessmentSettings,
  seed::AlbumRecommendationSeed,
  spotify_track_search_index::{SpotifyTrackQuery, SpotifyTrackSearchResult},
  types::{AlbumRecommendation, AlbumRecommendationSettings},
};
use crate::{
  context::ApplicationContext, files::file_metadata::file_name::FileName,
  profile::profile::ProfileId, proto, spotify::spotify_client::SpotifyTrackReference,
};
use anyhow::{anyhow, Error, Result};
use num_traits::Num;
use std::{collections::HashMap, sync::Arc};
use tonic::{async_trait, Request, Response, Status};
use tracing::error;

pub struct RecommendationService {
  recommendation_interactor: RecommendationInteractor,
}

impl RecommendationService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      recommendation_interactor: RecommendationInteractor::new(Arc::clone(&app_context)),
    }
  }
}

fn default_if_zero<T: Num>(value: T, default: T) -> T {
  if value.is_zero() {
    default
  } else {
    value
  }
}

impl TryFrom<proto::QuantileRankAlbumAssessmentSettings> for QuantileRankAlbumAssessmentSettings {
  type Error = Error;

  fn try_from(value: proto::QuantileRankAlbumAssessmentSettings) -> Result<Self, Self::Error> {
    let mut builder = QuantileRankAlbumAssessmentSettingsBuilder::default();
    if let Some(novelty_score) = value.novelty_score {
      builder.novelty_score(novelty_score);
    }
    if let Some(primary_genre_weight) = value.primary_genre_weight {
      builder.primary_genre_weight(primary_genre_weight);
    }
    if let Some(secondary_genre_weight) = value.secondary_genre_weight {
      builder.secondary_genre_weight(secondary_genre_weight);
    }
    if let Some(descriptor_weight) = value.descriptor_weight {
      builder.descriptor_weight(descriptor_weight);
    }
    if let Some(rating_weight) = value.rating_weight {
      builder.rating_weight(rating_weight);
    }
    if let Some(rating_count_weight) = value.rating_count_weight {
      builder.rating_count_weight(rating_count_weight);
    }
    if let Some(descriptor_count_weight) = value.descriptor_count_weight {
      builder.descriptor_count_weight(descriptor_count_weight);
    }
    if let Some(credit_tag_weight) = value.credit_tag_weight {
      builder.credit_tag_weight(credit_tag_weight);
    }
    Ok(builder.build()?)
  }
}

impl From<proto::EmbeddingSimilarityAlbumAssessmentSettings>
  for EmbeddingSimilarityAlbumAssessmentSettings
{
  fn from(value: proto::EmbeddingSimilarityAlbumAssessmentSettings) -> Self {
    Self {
      embedding_key: value.embedding_key,
    }
  }
}

impl TryFrom<proto::RerankedEmbeddingSimilarityAlbumAssessmentSettings>
  for RerankedEmbeddingSimilarityAlbumAssessmentSettings
{
  type Error = Error;

  fn try_from(
    value: proto::RerankedEmbeddingSimilarityAlbumAssessmentSettings,
  ) -> Result<Self, Self::Error> {
    let embedding_similarity_settings = EmbeddingSimilarityAlbumAssessmentSettings::from(
      value
        .embedding_similarity_settings
        .ok_or_else(|| anyhow!("Embedding similarity settings not provided"))?,
    );
    let quantile_rank_settings = value.quantile_rank_settings.map_or_else(
      || Ok(QuantileRankAlbumAssessmentSettings::default()),
      QuantileRankAlbumAssessmentSettings::try_from,
    )?;
    Ok(Self {
      embedding_similarity_settings,
      quantile_rank_settings,
      min_embedding_candidate_count: value.min_embedding_candidate_count,
    })
  }
}

impl TryFrom<proto::AlbumAssessmentSettings> for AlbumAssessmentSettings {
  type Error = Error;

  fn try_from(value: proto::AlbumAssessmentSettings) -> Result<Self, Self::Error> {
    match value.settings {
      Some(proto::album_assessment_settings::Settings::QuantileRankSettings(settings)) => Ok(
        Self::QuantileRank(QuantileRankAlbumAssessmentSettings::try_from(settings)?),
      ),
      Some(proto::album_assessment_settings::Settings::EmbeddingSimilaritySettings(settings)) => {
        Ok(Self::EmbeddingSimilarity(settings.into()))
      }
      Some(proto::album_assessment_settings::Settings::RerankedEmbeddingSimilaritySettings(
        settings,
      )) => Ok(Self::RerankedEmbeddingSimilarity(
        RerankedEmbeddingSimilarityAlbumAssessmentSettings::try_from(settings)?,
      )),

      None => Err(anyhow::anyhow!("Settings not provided")),
    }
  }
}

impl TryFrom<proto::AlbumRecommendationSettings> for AlbumRecommendationSettings {
  type Error = Error;

  fn try_from(value: proto::AlbumRecommendationSettings) -> Result<Self, Self::Error> {
    let default_count = AlbumRecommendationSettings::default().count;
    Ok(Self {
      count: value
        .count
        .map(|count| default_if_zero(count, default_count))
        .unwrap_or(default_count),
      include_primary_genres: value.include_primary_genres,
      include_secondary_genres: value.include_secondary_genres,
      include_languages: value.include_languages,
      exclude_primary_genres: value.exclude_primary_genres,
      exclude_secondary_genres: value.exclude_secondary_genres,
      exclude_languages: value.exclude_languages,
      include_descriptors: value.include_descriptors,
      exclude_descriptors: value.exclude_descriptors,
      min_release_year: value.min_release_year,
      max_release_year: value.max_release_year,
      exclude_known_artists: value.exclude_known_artists,
    })
  }
}

impl From<AlbumRecommendation> for proto::AlbumRecommendation {
  fn from(value: AlbumRecommendation) -> Self {
    Self {
      album: Some(value.album.into()),
      assessment: Some(proto::AlbumAssessment {
        score: value.assessment.score,
        metadata: value.assessment.metadata.unwrap_or_default(),
      }),
    }
  }
}

impl From<QuantileRankAlbumAssessmentSettings> for proto::QuantileRankAlbumAssessmentSettings {
  fn from(value: QuantileRankAlbumAssessmentSettings) -> Self {
    Self {
      novelty_score: Some(value.novelty_score as f32),
      primary_genre_weight: Some(value.primary_genre_weight),
      secondary_genre_weight: Some(value.secondary_genre_weight),
      descriptor_weight: Some(value.descriptor_weight),
      rating_weight: Some(value.rating_weight),
      rating_count_weight: Some(value.rating_count_weight),
      descriptor_count_weight: Some(value.descriptor_count_weight),
      credit_tag_weight: Some(value.credit_tag_weight),
    }
  }
}

impl From<proto::SpotifyTrackIndexQuery> for SpotifyTrackQuery {
  fn from(value: proto::SpotifyTrackIndexQuery) -> Self {
    Self {
      include_spotify_ids: value.include_spotify_ids,
      include_album_file_names: value
        .include_album_file_names
        .into_iter()
        .filter_map(|name| FileName::try_from(name).ok())
        .collect(),
    }
  }
}

impl From<SpotifyTrackSearchResult> for proto::SearchSpotifyTrackIndexReply {
  fn from(value: SpotifyTrackSearchResult) -> Self {
    Self {
      tracks: value
        .tracks
        .into_iter()
        .map(Into::<SpotifyTrackReference>::into)
        .map(Into::into)
        .collect(),
      total: value.total as u32,
    }
  }
}

impl TryFrom<proto::AlbumRecommendationSeed> for AlbumRecommendationSeed {
  type Error = anyhow::Error;

  fn try_from(value: proto::AlbumRecommendationSeed) -> Result<Self> {
    match value.value {
      Some(proto::album_recommendation_seed::Value::ProfileId(profile_id)) => {
        Ok(Self::Profile(ProfileId::try_from(profile_id)?))
      }
      Some(proto::album_recommendation_seed::Value::Albums(albums)) => Ok(Self::Albums(
        albums
          .file_names
          .into_iter()
          .map(|(name, factor)| Ok((FileName::try_from(name)?, factor)))
          .collect::<Result<HashMap<FileName, u32>>>()?,
      )),
      None => Err(anyhow!("Seed not provided")),
    }
  }
}

#[async_trait]
impl proto::RecommendationService for RecommendationService {
  async fn assess_album(
    &self,
    request: Request<proto::AssessAlbumRequest>,
  ) -> Result<Response<proto::AssessAlbumReply>, Status> {
    let request = request.into_inner();
    let seed_request = request.seed.ok_or_else(|| {
      error!("Seed not provided");
      Status::invalid_argument("Seed not provided")
    })?;
    let seed = AlbumRecommendationSeed::try_from(seed_request).map_err(|e| {
      error!(error = e.to_string(), "Invalid seed");
      Status::invalid_argument(e.to_string())
    })?;
    let file_name = FileName::try_from(request.file_name).map_err(|e| {
      error!(error = e.to_string(), "Invalid album file name");
      Status::invalid_argument(e.to_string())
    })?;
    let settings: AlbumAssessmentSettings = match request.settings {
      Some(settings) => AlbumAssessmentSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => AlbumAssessmentSettings::QuantileRank(QuantileRankAlbumAssessmentSettings::default()),
    };
    let assessment = self
      .recommendation_interactor
      .assess_album(seed, &file_name, settings)
      .await
      .map_err(|e| {
        error!(error = e.to_string(), "Failed to assess album");
        Status::internal(e.to_string())
      })?;
    Ok(Response::new(proto::AssessAlbumReply {
      assessment: Some(proto::AlbumAssessment {
        score: assessment.score,
        metadata: assessment.metadata.unwrap_or(HashMap::new()),
      }),
    }))
  }

  async fn recommend_albums(
    &self,
    request: Request<proto::RecommendAlbumsRequest>,
  ) -> Result<Response<proto::RecommendAlbumsReply>, Status> {
    let request = request.into_inner();
    let seed_request = request.seed.ok_or_else(|| {
      error!("Seed not provided");
      Status::invalid_argument("Seed not provided")
    })?;
    let seed = AlbumRecommendationSeed::try_from(seed_request).map_err(|e| {
      error!(error = e.to_string(), "Invalid seed");
      Status::invalid_argument(e.to_string())
    })?;
    let assessment_settings = match request.assessment_settings {
      Some(settings) => AlbumAssessmentSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => AlbumAssessmentSettings::QuantileRank(QuantileRankAlbumAssessmentSettings::default()),
    };
    let recommendation_settings = match request.recommendation_settings {
      Some(settings) => AlbumRecommendationSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => AlbumRecommendationSettings::default(),
    };
    let recommendations = self
      .recommendation_interactor
      .recommend_albums(seed, assessment_settings, recommendation_settings)
      .await
      .map_err(|e| {
        error!(error = e.to_string(), "Failed to recommend albums");
        Status::internal(e.to_string())
      })?;
    Ok(Response::new(proto::RecommendAlbumsReply {
      recommendations: recommendations.into_iter().map(Into::into).collect(),
    }))
  }

  async fn default_quantile_rank_album_assessment_settings(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::DefaultQuantileRankAlbumAssessmentSettingsReply>, Status> {
    Ok(Response::new(
      proto::DefaultQuantileRankAlbumAssessmentSettingsReply {
        settings: Some(QuantileRankAlbumAssessmentSettings::default().into()),
      },
    ))
  }

  async fn draft_spotify_playlist(
    &self,
    request: Request<proto::DraftSpotifyPlaylistRequest>,
  ) -> Result<Response<proto::DraftSpotifyPlaylistReply>, Status> {
    let request = request.into_inner();
    let seed_request = request.seed.ok_or_else(|| {
      error!("Seed not provided");
      Status::invalid_argument("Seed not provided")
    })?;
    let seed = AlbumRecommendationSeed::try_from(seed_request).map_err(|e| {
      error!(error = e.to_string(), "Invalid seed");
      Status::invalid_argument(e.to_string())
    })?;
    let assessment_settings = match request.assessment_settings {
      Some(settings) => AlbumAssessmentSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => AlbumAssessmentSettings::QuantileRank(QuantileRankAlbumAssessmentSettings::default()),
    };
    let recommendation_settings = match request.recommendation_settings {
      Some(settings) => AlbumRecommendationSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => AlbumRecommendationSettings::default(),
    };
    let tracks = self
      .recommendation_interactor
      .draft_spotify_playlist(seed, assessment_settings, recommendation_settings)
      .await
      .map_err(|e| {
        error!(error = e.to_string(), "Failed to draft Spotify playlist");
        Status::internal(e.to_string())
      })?;

    Ok(Response::new(proto::DraftSpotifyPlaylistReply {
      tracks: tracks.into_iter().map(Into::into).collect(),
    }))
  }

  async fn create_spotify_playlist(
    &self,
    request: Request<proto::CreateSpotifyPlaylistRequest>,
  ) -> Result<Response<proto::CreateSpotifyPlaylistReply>, Status> {
    let request = request.into_inner();
    let seed_request = request.seed.ok_or_else(|| {
      error!("Seed not provided");
      Status::invalid_argument("Seed not provided")
    })?;
    let seed = AlbumRecommendationSeed::try_from(seed_request).map_err(|e| {
      error!(error = e.to_string(), "Invalid seed");
      Status::invalid_argument(e.to_string())
    })?;
    let assessment_settings = match request.assessment_settings {
      Some(settings) => AlbumAssessmentSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => AlbumAssessmentSettings::QuantileRank(QuantileRankAlbumAssessmentSettings::default()),
    };
    let recommendation_settings = match request.recommendation_settings {
      Some(settings) => AlbumRecommendationSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => AlbumRecommendationSettings::default(),
    };
    let name = request.name;
    let description = request.description;
    let (playlist_id, tracks) = self
      .recommendation_interactor
      .create_spotify_playlist(
        seed,
        assessment_settings,
        recommendation_settings,
        name,
        description,
      )
      .await
      .map_err(|e| {
        error!(error = e.to_string(), "Failed to create Spotify playlist");
        Status::internal(e.to_string())
      })?;

    Ok(Response::new(proto::CreateSpotifyPlaylistReply {
      playlist_id,
      tracks: tracks.into_iter().map(Into::into).collect(),
    }))
  }

  async fn search_spotify_track_index(
    &self,
    request: Request<proto::SearchSpotifyTrackIndexRequest>,
  ) -> Result<Response<proto::SearchSpotifyTrackIndexReply>, Status> {
    let request = request.into_inner();
    let result = self
      .recommendation_interactor
      .search_spotify_track(
        &request.query.map(Into::into).unwrap_or_default(),
        request.pagination.map(Into::into).as_ref(),
      )
      .await
      .map_err(|e| {
        error!(
          error = e.to_string(),
          "Failed to search Spotify track index"
        );
        Status::internal(e.to_string())
      })?;
    Ok(Response::new(result.into()))
  }
}
