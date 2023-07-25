use super::{
  quantile_rank_interactor::QuantileRankAlbumAssessmentSettings,
  recommendation_interactor::{AlbumAssessmentSettings, RecommendationInteractor},
};
use crate::{
  files::file_metadata::file_name::FileName, profile::profile::ProfileId, proto, settings::Settings,
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
      primary_genre_weight: value.primary_genre_weight,
      secondary_genre_weight: value.secondary_genre_weight,
      descriptor_weight: value.descriptor_weight,
    })
  }
}

impl TryFrom<proto::assess_album_request::Settings> for AlbumAssessmentSettings {
  type Error = Error;

  fn try_from(value: proto::assess_album_request::Settings) -> Result<Self, Self::Error> {
    match value {
      proto::assess_album_request::Settings::QuantileRankSettings(settings) => {
        QuantileRankAlbumAssessmentSettings::try_from(settings)
          .map(AlbumAssessmentSettings::QuantileRank)
      }
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
      score: assessment.score,
      metadata: assessment.metadata.unwrap_or(HashMap::new()),
    }))
  }
}
