use crate::{
  albums::{
    album_interactor::AlbumInteractor,
    album_read_model::AlbumReadModel,
    album_search_index::{AlbumEmbeddingSimilarirtySearchQuery, AlbumSearchQueryBuilder},
  },
  helpers::embedding::average_embedding,
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
use std::sync::Arc;
use tracing::{instrument, warn};

pub struct EmbeddingSimilarityInteractor {
  album_interactor: Arc<AlbumInteractor>,
}

#[derive(Clone, Debug)]
pub struct EmbeddingSimilarityAlbumAssessmentSettings {
  pub embedding_key: String,
}

impl EmbeddingSimilarityInteractor {
  pub fn new(album_interactor: Arc<AlbumInteractor>) -> Self {
    Self { album_interactor }
  }

  pub async fn get_average_seed_embedding(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    settings: &EmbeddingSimilarityAlbumAssessmentSettings,
  ) -> Result<Vec<f32>> {
    let album_embeddings = self
      .album_interactor
      .find_many_embeddings(seed_context.album_file_names(), &settings.embedding_key)
      .await?;
    Ok(average_embedding(
      album_embeddings
        .iter()
        .map(|embedding| {
          (
            &embedding.embedding,
            seed_context.get_factor(&embedding.file_name).unwrap_or(1),
          )
        })
        .collect(),
    ))
  }
}

#[derive(Clone, Debug)]
pub struct EmbeddingSimilarityAssessableAlbum(AlbumReadModel);

impl TryFrom<AlbumReadModel> for EmbeddingSimilarityAssessableAlbum {
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
  RecommendationMethodInteractor<
    EmbeddingSimilarityAssessableAlbum,
    EmbeddingSimilarityAlbumAssessmentSettings,
  > for EmbeddingSimilarityInteractor
{
  #[instrument(
    name = "EmbeddingSimilarityInteractor::assess_album",
    skip(self, seed_context)
  )]
  async fn assess_album(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    album_read_model: &EmbeddingSimilarityAssessableAlbum,
    settings: EmbeddingSimilarityAlbumAssessmentSettings,
  ) -> Result<AlbumAssessment> {
    let profile_embedding = self
      .get_average_seed_embedding(&seed_context, &settings)
      .await?;
    let mut search_result = self
      .album_interactor
      .embedding_similarity_search(&AlbumEmbeddingSimilarirtySearchQuery {
        embedding: profile_embedding,
        embedding_key: settings.embedding_key.clone(),
        filters: AlbumSearchQueryBuilder::default()
          .include_file_names(vec![album_read_model.0.file_name.clone()])
          .build()?,
        limit: 1,
      })
      .await?;
    let (_, score) = search_result.pop().ok_or_else(|| {
      warn!("Embeddings search returned no results");
      anyhow::anyhow!("Embeddings search returned no results")
    })?;
    Ok(AlbumAssessment {
      score,
      metadata: None,
    })
  }

  #[instrument(
    name = "EmbeddingSimilarityInteractor::recommend_albums",
    skip(self, seed_context)
  )]
  async fn recommend_albums(
    &self,
    seed_context: &AlbumRecommendationSeedContext,
    assessment_settings: EmbeddingSimilarityAlbumAssessmentSettings,
    recommendation_settings: AlbumRecommendationSettings,
  ) -> Result<Vec<AlbumRecommendation>> {
    let profile_embedding = self
      .get_average_seed_embedding(&seed_context, &assessment_settings)
      .await?;
    let search_query = recommendation_settings.to_search_query(&seed_context.albums)?;
    let similar_albums = self
      .album_interactor
      .embedding_similarity_search(&AlbumEmbeddingSimilarirtySearchQuery {
        embedding: profile_embedding,
        embedding_key: assessment_settings.embedding_key.clone(),
        filters: search_query,
        limit: recommendation_settings.count as usize,
      })
      .await?;
    Ok(
      similar_albums
        .into_iter()
        .map(|(album, score)| AlbumRecommendation {
          album,
          assessment: AlbumAssessment {
            score,
            metadata: None,
          },
        })
        .collect(),
    )
  }
}
