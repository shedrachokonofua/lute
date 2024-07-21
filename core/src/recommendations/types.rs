use crate::{
  albums::{
    album_read_model::AlbumReadModel,
    album_search_index::{AlbumSearchQuery, AlbumSearchQueryBuilder},
  },
  profile::profile::Profile,
};
use anyhow::Result;
use async_trait::async_trait;
use std::{cmp::Ordering, collections::HashMap};

#[derive(Clone, Debug)]
pub struct AlbumRecommendationSettings {
  pub count: u32,
  pub include_primary_genres: Vec<String>,
  pub include_secondary_genres: Vec<String>,
  pub include_languages: Vec<String>,
  pub exclude_primary_genres: Vec<String>,
  pub exclude_secondary_genres: Vec<String>,
  pub include_descriptors: Vec<String>,
  pub exclude_descriptors: Vec<String>,
  pub exclude_languages: Vec<String>,
  pub min_release_year: Option<u32>,
  pub max_release_year: Option<u32>,
  pub exclude_known_artists: Option<bool>,
}

impl Default for AlbumRecommendationSettings {
  fn default() -> Self {
    Self {
      count: 10,
      include_primary_genres: vec![],
      include_secondary_genres: vec![],
      include_languages: vec![],
      exclude_primary_genres: vec![],
      exclude_secondary_genres: vec![],
      exclude_languages: vec![],
      min_release_year: None,
      max_release_year: None,
      exclude_known_artists: Some(true),
      include_descriptors: vec![],
      exclude_descriptors: vec![],
    }
  }
}
impl AlbumRecommendationSettings {
  pub fn to_search_query(
    &self,
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
  ) -> Result<AlbumSearchQuery> {
    let mut search_query_builder = AlbumSearchQueryBuilder::default();
    search_query_builder
      .exclude_file_names(profile.albums.keys().cloned().collect::<Vec<_>>())
      .include_primary_genres(self.include_primary_genres.clone())
      .include_secondary_genres(self.include_secondary_genres.clone())
      .include_languages(self.include_languages.clone())
      .exclude_primary_genres(self.exclude_primary_genres.clone())
      .exclude_secondary_genres(self.exclude_secondary_genres.clone())
      .exclude_languages(self.exclude_languages.clone())
      .min_release_year(self.min_release_year)
      .max_release_year(self.max_release_year)
      .min_primary_genre_count(1)
      .min_secondary_genre_count(1)
      .min_descriptor_count(5);
    if self.exclude_known_artists.unwrap_or(false) {
      search_query_builder.exclude_artists(
        profile_albums
          .iter()
          .flat_map(|album| album.artists.clone())
          .map(|artist| artist.file_name)
          .collect::<Vec<_>>(),
      );
    }
    Ok(search_query_builder.build()?)
  }
}

#[derive(Clone, Debug)]
pub struct AlbumAssessment {
  pub score: f32,
  pub metadata: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug)]
pub struct AlbumRecommendation {
  pub album: AlbumReadModel,
  pub assessment: AlbumAssessment,
}

impl PartialEq for AlbumRecommendation {
  fn eq(&self, other: &Self) -> bool {
    self.assessment.score == other.assessment.score
  }
}

impl Eq for AlbumRecommendation {}

impl PartialOrd for AlbumRecommendation {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.assessment.score.partial_cmp(&other.assessment.score)
  }
}

impl Ord for AlbumRecommendation {
  fn cmp(&self, other: &Self) -> Ordering {
    self
      .assessment
      .score
      .partial_cmp(&other.assessment.score)
      .unwrap_or(Ordering::Equal)
  }
}

#[async_trait]
pub trait RecommendationMethodInteractor<
  TAssessableAlbum: TryFrom<AlbumReadModel>,
  TAlbumAssessmentSettings,
>
{
  async fn assess_album(
    &self,
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
    album: &TAssessableAlbum,
    settings: TAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment>;

  async fn recommend_albums(
    &self,
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
    assessment_settings: TAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>>;
}
