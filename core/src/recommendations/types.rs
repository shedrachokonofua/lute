use crate::{
  albums::album_read_model_repository::AlbumReadModel, profile::profile_summary::ProfileSummary,
};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct AlbumAssessment {
  pub score: f32,
  pub metadata: Option<HashMap<String, String>>,
}

#[async_trait]
pub trait RecommendationMethodInteractor<TAlbumAssessmentSettings> {
  async fn assess_album(
    &self,
    profile_summary: &ProfileSummary,
    album_read_model: &AlbumReadModel,
    settings: TAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment>;
}
