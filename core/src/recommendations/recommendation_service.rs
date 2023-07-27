use super::{
  quantile_rank_interactor::{
    QuantileRankAlbumAssessmentSettings, QuantileRankAlbumAssessmentSettingsBuilder,
  },
  recommendation_interactor::{AlbumAssessmentSettings, RecommendationInteractor},
  types::{AlbumRecommendation, AlbumRecommendationSettings},
};
use crate::{
  files::file_metadata::file_name::FileName,
  profile::profile::ProfileId,
  proto::{self},
  settings::Settings,
};
use anyhow::Error;
use num_traits::Num;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::{collections::HashMap, sync::Arc};
use tonic::{async_trait, Request, Response, Status};
use tracing::error;

pub struct RecommendationService {
  recommendation_interactor: RecommendationInteractor,
}

impl RecommendationService {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
  ) -> Self {
    Self {
      recommendation_interactor: RecommendationInteractor::new(settings, redis_connection_pool),
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
      builder.novelty_score(default_if_zero(
        novelty_score.into(),
        QuantileRankAlbumAssessmentSettings::default().novelty_score,
      ));
    }
    if let Some(primary_genre_weight) = value.primary_genre_weight {
      builder.primary_genre_weight(default_if_zero(
        primary_genre_weight.into(),
        QuantileRankAlbumAssessmentSettings::default().primary_genre_weight,
      ));
    }
    if let Some(secondary_genre_weight) = value.secondary_genre_weight {
      builder.secondary_genre_weight(default_if_zero(
        secondary_genre_weight.into(),
        QuantileRankAlbumAssessmentSettings::default().secondary_genre_weight,
      ));
    }
    if let Some(descriptor_weight) = value.descriptor_weight {
      builder.descriptor_weight(default_if_zero(
        descriptor_weight.into(),
        QuantileRankAlbumAssessmentSettings::default().descriptor_weight,
      ));
    }
    if let Some(rating_weight) = value.rating_weight {
      builder.rating_weight(default_if_zero(
        rating_weight.into(),
        QuantileRankAlbumAssessmentSettings::default().rating_weight,
      ));
    }
    if let Some(rating_count_weight) = value.rating_count_weight {
      builder.rating_count_weight(default_if_zero(
        rating_count_weight.into(),
        QuantileRankAlbumAssessmentSettings::default().rating_count_weight,
      ));
    }
    Ok(builder.build()?)
  }
}

impl TryFrom<proto::AlbumAssessmentSettings> for AlbumAssessmentSettings {
  type Error = Error;

  fn try_from(value: proto::AlbumAssessmentSettings) -> Result<Self, Self::Error> {
    match value.settings {
      Some(proto::album_assessment_settings::Settings::QuantileRankSettings(settings)) => Ok(
        Self::QuantileRank(QuantileRankAlbumAssessmentSettings::try_from(settings)?),
      ),
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
        .map(|count| default_if_zero(count.into(), default_count))
        .unwrap_or(default_count),
      include_primary_genres: value.include_primary_genres,
      include_secondary_genres: value.include_secondary_genres,
      exclude_primary_genres: value.exclude_primary_genres,
      exclude_secondary_genres: value.exclude_secondary_genres,
    })
  }
}

impl From<AlbumRecommendation> for proto::AlbumRecommendation {
  fn from(value: AlbumRecommendation) -> Self {
    Self {
      album: Some(value.album.into()),
      assessment: Some(proto::AlbumAssessment {
        score: value.assessment.score,
        metadata: value.assessment.metadata.unwrap_or(HashMap::new()),
      }),
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
    let profile_id = ProfileId::try_from(request.profile_id).map_err(|e| {
      error!(error = e.to_string(), "Invalid profile ID");
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
      .assess_album(&profile_id, &file_name, settings)
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
    let profile_id = ProfileId::try_from(request.profile_id).map_err(|e| {
      error!(error = e.to_string(), "Invalid profile ID");
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
      .recommend_albums(&profile_id, assessment_settings, recommendation_settings)
      .await
      .map_err(|e| {
        error!(error = e.to_string(), "Failed to recommend albums");
        Status::internal(e.to_string())
      })?;
    Ok(Response::new(proto::RecommendAlbumsReply {
      recommendations: recommendations.into_iter().map(Into::into).collect(),
    }))
  }
}
