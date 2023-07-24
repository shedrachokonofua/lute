use crate::{files::file_metadata::file_name::FileName, profile::profile::ProfileId};
use anyhow::Result;
use std::collections::HashMap;

pub struct AlbumAssessment {
  pub score: f32,
  pub metadata: Option<HashMap<String, String>>,
}

pub trait RecommendationMethodInteractor<TAlbumAssessmentSettings> {
  fn assess_album(
    &self,
    profile_id: &ProfileId,
    album_file_name: &FileName,
    settings: TAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment>;
}
