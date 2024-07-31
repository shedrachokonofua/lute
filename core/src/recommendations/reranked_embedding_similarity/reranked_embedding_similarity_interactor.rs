use crate::{
  albums::album_read_model::AlbumReadModel,
  recommendations::{
    embedding_similarity::embedding_similarity_interactor::{
      EmbeddingSimilarityAlbumAssessmentSettings, EmbeddingSimilarityAssessableAlbum,
      EmbeddingSimilarityInteractor,
    },
    quantile_ranking::quantile_rank_interactor::{
      QuantileRankAlbumAssessmentSettings, QuantileRankAssessableAlbum, QuantileRankInteractor,
    },
    seed::AlbumRecommendationSeedContext,
    types::{
      AlbumAssessment, AlbumRecommendation, AlbumRecommendationSettings,
      RecommendationMethodInteractor,
    },
  },
};
use anyhow::Result;
use std::{cmp::max, collections::HashMap, sync::Arc};
use tonic::async_trait;
use tracing::instrument;

#[derive(Clone, Debug)]
pub struct RerankedEmbeddingSimilarityAlbumAssessmentSettings {
  pub embedding_similarity_settings: EmbeddingSimilarityAlbumAssessmentSettings,
  pub quantile_rank_settings: QuantileRankAlbumAssessmentSettings,
  pub min_embedding_candidate_count: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct RerankedEmbeddingSimilarityAssessableAlbum(AlbumReadModel);

impl TryFrom<AlbumReadModel> for RerankedEmbeddingSimilarityAssessableAlbum {
  type Error = anyhow::Error;

  fn try_from(album_read_model: AlbumReadModel) -> Result<Self, Self::Error> {
    match (
      EmbeddingSimilarityAssessableAlbum::try_from(album_read_model.clone()),
      QuantileRankAssessableAlbum::try_from(album_read_model.clone()),
    ) {
      (Ok(_), Ok(_)) => Ok(Self(album_read_model)),
      _ => Err(anyhow::anyhow!(
        "Failed to convert to RerankedEmbeddingSimilarityAssessableAlbum"
      )),
    }
  }
}

pub struct RerankedEmbeddingSimilarityInteractor {
  embedding_similarity_interactor: Arc<EmbeddingSimilarityInteractor>,
  quantile_rank_interactor: Arc<QuantileRankInteractor>,
}

impl RerankedEmbeddingSimilarityInteractor {
  pub fn new(
    embedding_similarity_interactor: Arc<EmbeddingSimilarityInteractor>,
    quantile_rank_interactor: Arc<QuantileRankInteractor>,
  ) -> Self {
    Self {
      embedding_similarity_interactor,
      quantile_rank_interactor,
    }
  }
}

#[async_trait]
impl
  RecommendationMethodInteractor<
    RerankedEmbeddingSimilarityAssessableAlbum,
    RerankedEmbeddingSimilarityAlbumAssessmentSettings,
  > for RerankedEmbeddingSimilarityInteractor
{
  #[instrument(
    name = "RerankedEmbeddingSimilarityInteractor::assess_album",
    skip(self, seed_context)
  )]
  async fn assess_album(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    album: &RerankedEmbeddingSimilarityAssessableAlbum,
    settings: RerankedEmbeddingSimilarityAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    self
      .quantile_rank_interactor
      .assess_album(
        seed_context,
        &(QuantileRankAssessableAlbum::try_from(album.0.clone())?),
        settings.quantile_rank_settings,
      )
      .await
  }
  #[instrument(
    name = "RerankedEmbeddingSimilarityInteractor::recommend_albums",
    skip(self, seed_context)
  )]
  async fn recommend_albums(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    assessment_settings: RerankedEmbeddingSimilarityAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let mut embedding_similiarity_recommendation_settings = recommendation_settings.clone();
    embedding_similiarity_recommendation_settings.count = max(
      embedding_similiarity_recommendation_settings.count * 2,
      assessment_settings
        .min_embedding_candidate_count
        .unwrap_or(50),
    );
    let embedding_similiarity_recommendations = self
      .embedding_similarity_interactor
      .recommend_albums(
        seed_context,
        assessment_settings.embedding_similarity_settings,
        embedding_similiarity_recommendation_settings,
      )
      .await?;

    let mut embedding_similarity_metadata = embedding_similiarity_recommendations
      .iter()
      .enumerate()
      .map(|(i, recommendation)| {
        (
          recommendation.album.file_name.clone(),
          HashMap::from([
            ("embedding_similarity_rank".to_string(), i.to_string()),
            (
              "embedding_similarity_score".to_string(),
              recommendation.assessment.score.to_string(),
            ),
          ]),
        )
      })
      .collect::<HashMap<_, _>>();
    let similar_albums = embedding_similiarity_recommendations
      .into_iter()
      .map(|r| r.album)
      .collect::<Vec<_>>();

    let mut recommendations = self
      .quantile_rank_interactor
      .rank_albums(
        seed_context,
        assessment_settings.quantile_rank_settings,
        recommendation_settings,
        similar_albums,
      )
      .await?;

    for recommendation in recommendations.iter_mut() {
      if recommendation.assessment.metadata.is_none() {
        recommendation.assessment.metadata = Some(HashMap::new());
      }
      for (key, value) in embedding_similarity_metadata
        .remove(&recommendation.album.file_name)
        .unwrap_or_default()
        .into_iter()
      {
        recommendation
          .assessment
          .metadata
          .as_mut()
          .unwrap()
          .insert(key, value);
      }
    }

    Ok(recommendations)
  }
}
