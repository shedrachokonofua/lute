use crate::{files::file_metadata::file_name::FileName, helpers::redisearch::SearchPagination};
use anyhow::Result;
use async_trait::async_trait;
use derive_builder::Builder;
use serde_derive::{Deserialize, Serialize};

use super::album_read_model::AlbumReadModel;

#[derive(Default, Builder, Debug)]
#[builder(setter(into), default)]
pub struct AlbumSearchQuery {
  pub text: Option<String>,
  pub exact_name: Option<String>,
  pub include_file_names: Vec<FileName>,
  pub exclude_file_names: Vec<FileName>,
  pub include_artists: Vec<String>,
  pub exclude_artists: Vec<String>,
  pub include_primary_genres: Vec<String>,
  pub exclude_primary_genres: Vec<String>,
  pub include_secondary_genres: Vec<String>,
  pub exclude_secondary_genres: Vec<String>,
  pub include_languages: Vec<String>,
  pub exclude_languages: Vec<String>,
  pub include_descriptors: Vec<String>,
  pub exclude_descriptors: Vec<String>,
  pub min_primary_genre_count: Option<usize>,
  pub min_secondary_genre_count: Option<usize>,
  pub min_descriptor_count: Option<usize>,
  pub min_release_year: Option<u32>,
  pub max_release_year: Option<u32>,
  pub include_duplicates: Option<bool>,
}

#[derive(Debug)]
pub struct AlbumSearchResult {
  pub albums: Vec<AlbumReadModel>,
  pub total: usize,
}

#[derive(Debug)]
pub struct AlbumEmbeddingSimilarirtySearchQuery {
  pub embedding: Vec<f32>,
  pub embedding_key: String,
  pub filters: AlbumSearchQuery,
  pub limit: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct AlbumEmbedding {
  pub file_name: FileName,
  pub key: String,
  pub embedding: Vec<f32>,
}

pub fn embedding_to_bytes(embedding: &Vec<f32>) -> Vec<u8> {
  embedding
    .iter()
    .flat_map(|f| f.to_ne_bytes().to_vec())
    .collect()
}

impl AlbumEmbedding {
  pub fn embedding_bytes(&self) -> Vec<u8> {
    embedding_to_bytes(&self.embedding)
  }
}

#[async_trait]
pub trait AlbumSearchIndex {
  async fn put(&self, album: AlbumReadModel) -> Result<()>;
  async fn delete(&self, file_name: &FileName) -> Result<()>;
  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>>;
  async fn search(
    &self,
    query: &AlbumSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<AlbumSearchResult>;
  async fn get_embeddings(&self, file_name: &FileName) -> Result<Vec<AlbumEmbedding>>;
  async fn find_many_embeddings(
    &self,
    file_names: Vec<FileName>,
    key: &str,
  ) -> Result<Vec<AlbumEmbedding>>;
  async fn find_embedding(&self, file_name: &FileName, key: &str)
    -> Result<Option<AlbumEmbedding>>;
  async fn put_embedding(&self, embedding: &AlbumEmbedding) -> Result<()>;
  async fn delete_embedding(&self, file_name: &FileName, key: &str) -> Result<()>;
  async fn embedding_similarity_search(
    &self,
    query: &AlbumEmbeddingSimilarirtySearchQuery,
  ) -> Result<Vec<(AlbumReadModel, f32)>>;
}
