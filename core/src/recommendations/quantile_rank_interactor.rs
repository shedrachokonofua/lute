use super::{
  quantile_rank_assessment::QuantileRankAlbumAssessmentContext,
  types::{
    AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings,
    RecommendationMethodInteractor,
  },
};
use crate::{
  albums::album_read_model_repository::{
    AlbumReadModel, AlbumReadModelRepository, AlbumSearchQueryBuilder,
  },
  helpers::bounded_min_heap::BoundedMinHeap,
  profile::profile::Profile,
};
use anyhow::Result;
use async_trait::async_trait;
use derive_builder::Builder;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{instrument, warn};

#[derive(Builder, Clone, Debug)]
#[builder(setter(into), default)]
pub struct QuantileRankAlbumAssessmentSettings {
  pub primary_genre_weight: u32,
  pub secondary_genre_weight: u32,
  pub descriptor_weight: u32,
  pub rating_weight: u32,
  pub rating_count_weight: u32,
  pub novelty_score: f64,
  pub descriptor_count_weight: u32,
  pub credit_tag_weight: u32,
}

impl Default for QuantileRankAlbumAssessmentSettings {
  fn default() -> Self {
    Self {
      primary_genre_weight: 4,
      secondary_genre_weight: 2,
      descriptor_weight: 7,
      rating_weight: 2,
      rating_count_weight: 1,
      novelty_score: 0.5,
      descriptor_count_weight: 2,
      credit_tag_weight: 1,
    }
  }
}

pub struct QuantileRankInteractor {
  album_read_model_repository: AlbumReadModelRepository,
}

impl QuantileRankInteractor {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      album_read_model_repository: AlbumReadModelRepository::new(Arc::clone(
        &redis_connection_pool,
      )),
    }
  }
}

#[derive(Clone, Debug)]
pub struct QuantileRankAssessableAlbum(AlbumReadModel);

impl TryFrom<AlbumReadModel> for QuantileRankAssessableAlbum {
  type Error = anyhow::Error;

  fn try_from(album_read_model: AlbumReadModel) -> Result<Self, Self::Error> {
    if album_read_model.descriptors.len() < 5 {
      return Err(anyhow::anyhow!("Not enough descriptors"));
    }

    Ok(Self(album_read_model))
  }
}

#[async_trait]
impl
  RecommendationMethodInteractor<QuantileRankAssessableAlbum, QuantileRankAlbumAssessmentSettings>
  for QuantileRankInteractor
{
  #[instrument(name = "QuantileRankInteractor::assess_album", skip(self))]
  async fn assess_album(
    &self,
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
    album_read_model: &QuantileRankAssessableAlbum,
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    QuantileRankAlbumAssessmentContext::new(profile, profile_albums, settings)
      .assess(&album_read_model.0)
  }

  #[instrument(name = "QuantileRankInteractor::recommend_albums", skip(self))]
  async fn recommend_albums(
    &self,
    profile: &Profile,
    profile_albums: &[AlbumReadModel],
    assessment_settings: QuantileRankAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let mut search_query_builder = AlbumSearchQueryBuilder::default();
    search_query_builder
      .exclude_file_names(profile.albums.keys().cloned().collect::<Vec<_>>())
      .include_primary_genres(recommendation_settings.include_primary_genres)
      .include_secondary_genres(recommendation_settings.include_secondary_genres)
      .include_languages(recommendation_settings.include_languages)
      .exclude_primary_genres(recommendation_settings.exclude_primary_genres)
      .exclude_secondary_genres(recommendation_settings.exclude_secondary_genres)
      .exclude_languages(recommendation_settings.exclude_languages)
      .min_release_year(recommendation_settings.min_release_year)
      .max_release_year(recommendation_settings.max_release_year)
      .min_primary_genre_count(1)
      .min_secondary_genre_count(1)
      .min_descriptor_count(5);
    if recommendation_settings
      .exclude_known_artists
      .unwrap_or(false)
    {
      search_query_builder.exclude_artists(
        profile_albums
          .iter()
          .flat_map(|album| album.artists.clone())
          .map(|artist| artist.file_name.to_string())
          .collect::<Vec<_>>(),
      );
    }
    let search_query = search_query_builder.build()?;
    let albums = self
      .album_read_model_repository
      .search(&search_query)
      .await?;
    let context =
      QuantileRankAlbumAssessmentContext::new(profile, profile_albums, assessment_settings);
    let mut result_heap = BoundedMinHeap::new(recommendation_settings.count as usize);
    let (recommendation_sender, mut recommendation_receiver) = mpsc::unbounded_channel();
    rayon::spawn(move || {
      albums
        .par_iter()
        .for_each(|album| match context.assess(album) {
          Ok(assessment) => {
            let recommendation = AlbumRecommendation {
              album: album.clone(),
              assessment,
            };
            recommendation_sender.send(recommendation).unwrap();
          }
          Err(error) => {
            warn!("Error assessing album: {}", error);
          }
        });
    });
    while let Some(recommendation) = recommendation_receiver.recv().await {
      result_heap.push(recommendation);
    }
    let recommendations = result_heap.drain_sorted_desc();
    Ok(recommendations)
  }
}
