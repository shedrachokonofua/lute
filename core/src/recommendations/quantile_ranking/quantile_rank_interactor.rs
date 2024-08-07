use super::{
  bounded_min_heap::BoundedMinHeap, quantile_rank_assessment::QuantileRankAlbumAssessmentContext,
};
use crate::{
  albums::{album_interactor::AlbumInteractor, album_read_model::AlbumReadModel},
  helpers::redisearch::SearchPagination,
  recommendations::{
    seed::AlbumRecommendationSeedContext,
    types::{
      AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings,
      RecommendationMethodInteractor,
    },
  },
};
use anyhow::Result;
use async_trait::async_trait;
use derive_builder::Builder;
use rayon::{iter::ParallelDrainRange, prelude::ParallelIterator};
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
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
      novelty_score: 0.2,
      descriptor_count_weight: 2,
      credit_tag_weight: 1,
    }
  }
}

pub struct QuantileRankInteractor {
  album_interactor: Arc<AlbumInteractor>,
}

impl QuantileRankInteractor {
  pub fn new(album_interactor: Arc<AlbumInteractor>) -> Self {
    Self { album_interactor }
  }

  #[instrument(name = "QuantileRankInteractor::rank_albums", skip(self, seed_context))]
  pub async fn rank_albums(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    assessment_settings: QuantileRankAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
    albums: Vec<AlbumReadModel>,
  ) -> Result<Vec<AlbumRecommendation>> {
    let mut albums = albums;
    let context = QuantileRankAlbumAssessmentContext::new(seed_context, assessment_settings);
    let mut result_heap = BoundedMinHeap::new(recommendation_settings.count as usize);
    let (recommendation_sender, mut recommendation_receiver) = unbounded_channel();
    rayon::spawn(move || {
      albums
        .par_drain(..)
        .for_each(|album| match context.assess(&album) {
          Ok(assessment) => {
            if let Err(e) = recommendation_sender.send(AlbumRecommendation { album, assessment }) {
              warn!("Error sending recommendation: {}", e);
            }
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
  #[instrument(
    name = "QuantileRankInteractor::assess_album",
    skip(self, seed_context, album_read_model)
  )]
  async fn assess_album(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    album_read_model: &QuantileRankAssessableAlbum,
    settings: QuantileRankAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    QuantileRankAlbumAssessmentContext::new(seed_context, settings).assess(&album_read_model.0)
  }

  #[instrument(
    name = "QuantileRankInteractor::recommend_albums",
    skip(self, seed_context)
  )]
  async fn recommend_albums(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    assessment_settings: QuantileRankAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let search_query = recommendation_settings.to_search_query(&seed_context.albums)?;
    let search_results = self
      .album_interactor
      .search(
        &search_query,
        Some(&SearchPagination {
          offset: None,
          limit: Some(100000),
        }),
      )
      .await?;
    self
      .rank_albums(
        seed_context,
        assessment_settings,
        recommendation_settings,
        search_results.albums,
      )
      .await
  }
}
