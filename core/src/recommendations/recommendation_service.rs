use super::{
  quantile_rank_interactor::QuantileRankAlbumAssessmentSettings,
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

impl TryFrom<proto::QuantileRankAlbumAssessmentSettings> for QuantileRankAlbumAssessmentSettings {
  type Error = Error;

  fn try_from(value: proto::QuantileRankAlbumAssessmentSettings) -> Result<Self, Self::Error> {
    Ok(Self {
      primary_genre_weight: if value.primary_genre_weight == 0 {
        6
      } else {
        value.primary_genre_weight
      },
      secondary_genre_weight: if value.secondary_genre_weight == 0 {
        3
      } else {
        value.secondary_genre_weight
      },
      descriptor_weight: if value.descriptor_weight == 0 {
        20
      } else {
        value.descriptor_weight
      },
      novelty_score: if value.novelty_score == 0.0 {
        0.5
      } else {
        value.novelty_score as f64
      },
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
      None => Err(anyhow::anyhow!("Settings not provided")),
    }
  }
}

impl TryFrom<proto::AlbumRecommendationSettings> for AlbumRecommendationSettings {
  type Error = Error;

  fn try_from(value: proto::AlbumRecommendationSettings) -> Result<Self, Self::Error> {
    Ok(Self {
      count: if value.count == 0 { 50 } else { value.count },
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
    let settings = match request.settings {
      Some(settings) => AlbumAssessmentSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => {
        error!("Settings not provided");
        return Err(Status::invalid_argument("Settings not provided"));
      }
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
      None => {
        error!("Settings not provided");
        return Err(Status::invalid_argument("Settings not provided"));
      }
    };
    let recommendation_settings = match request.recommendation_settings {
      Some(settings) => AlbumRecommendationSettings::try_from(settings).map_err(|e| {
        error!(error = e.to_string(), "Invalid settings");
        Status::invalid_argument(e.to_string())
      })?,
      None => {
        error!("Settings not provided");
        return Err(Status::invalid_argument("Settings not provided"));
      }
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
