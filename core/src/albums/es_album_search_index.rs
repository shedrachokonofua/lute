use super::{
  album_read_model::{
    AlbumReadModel, AlbumReadModelArtist, AlbumReadModelCredit, AlbumReadModelTrack,
  },
  album_search_index::{
    AlbumEmbeddingSimilarirtySearchQuery, AlbumSearchIndex, AlbumSearchQuery, AlbumSearchResult,
  },
};
use crate::{
  files::file_metadata::file_name::FileName,
  helpers::{
    elasticsearch_index::{ElasticsearchIndex, ElasticsearchResult},
    embedding::EmbeddingDocument,
    redisearch::SearchPagination,
  },
};
use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use elasticsearch::Elasticsearch;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tonic::async_trait;
use tracing::error;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct EsAlbumReadModel {
  pub name: String,
  pub file_name: FileName,
  pub rating: f32,
  pub rating_count: u32,
  pub artists: Vec<AlbumReadModelArtist>,
  pub artist_count: u32,
  pub primary_genres: Vec<String>,
  pub primary_genre_count: u32,
  pub secondary_genres: Vec<String>,
  pub secondary_genre_count: u32,
  pub descriptors: Vec<String>,
  pub descriptor_count: u32,
  pub tracks: Vec<AlbumReadModelTrack>,
  pub track_count: u32,
  pub release_date: Option<NaiveDate>,
  pub release_year: Option<u32>,
  pub languages: Vec<String>,
  pub language_count: u32,
  pub credits: Vec<AlbumReadModelCredit>,
  pub credit_count: u32,
  pub duplicate_of: Option<FileName>,
  pub is_duplicate: bool,
  pub duplicates: Vec<FileName>,
  pub duplicate_count: u32,
  pub cover_image_url: Option<String>,
  pub spotify_id: Option<String>,
}

impl From<AlbumReadModel> for EsAlbumReadModel {
  fn from(album: AlbumReadModel) -> Self {
    Self {
      name: album.name,
      file_name: album.file_name,
      rating: album.rating,
      rating_count: album.rating_count,
      artist_count: album.artists.len() as u32,
      artists: album.artists,
      primary_genre_count: album.primary_genres.len() as u32,
      primary_genres: album.primary_genres,
      secondary_genre_count: album.secondary_genres.len() as u32,
      secondary_genres: album.secondary_genres,
      descriptor_count: album.descriptors.len() as u32,
      descriptors: album.descriptors,
      track_count: album.tracks.len() as u32,
      tracks: album.tracks,
      release_date: album.release_date,
      release_year: album.release_date.map(|d| d.year() as u32),
      language_count: album.languages.len() as u32,
      languages: album.languages,
      credit_count: album.credits.len() as u32,
      credits: album.credits,
      is_duplicate: album.duplicate_of.is_some(),
      duplicate_of: album.duplicate_of,
      duplicate_count: album.duplicates.len() as u32,
      duplicates: album.duplicates,
      cover_image_url: album.cover_image_url,
      spotify_id: album.spotify_id,
    }
  }
}

impl AlbumSearchQuery {
  pub fn to_es_query(&self) -> Value {
    let mut query = json!({
      "bool": {
        "must": [],
        "must_not": []
      }
    });

    if let Some(text) = &self.text {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "multi_match": {
          "query": text,
          "fields": ["name", "artists.name"],
          "fuzziness": "AUTO"
        }
      }));
    }

    if let Some(exact_name) = &self.exact_name {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "term": {
          "name.keyword": exact_name
        }
      }));
    }

    if !self.include_file_names.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "file_name.keyword": self.include_file_names
            .iter()
            .map(|file_name| file_name.to_string())
            .collect::<Vec<String>>()
        }
      }));
    }

    if !self.exclude_file_names.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "file_name.keyword": self.exclude_file_names
              .iter()
              .map(|file_name| file_name.to_string())
              .collect::<Vec<String>>()
          }
        }));
    }

    if !self.include_artists.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "artists.name.keyword": self.include_artists
        }
      }));
    }

    if !self.exclude_artists.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "artists.name.keyword": self.exclude_artists
          }
        }));
    }

    if !self.include_primary_genres.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "primary_genres.keyword": self.include_primary_genres
        }
      }));
    }

    if !self.exclude_primary_genres.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "primary_genres.keyword": self.exclude_primary_genres
          }
        }));
    }

    if !self.include_secondary_genres.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "secondary_genres.keyword": self.include_secondary_genres
        }
      }));
    }

    if !self.exclude_secondary_genres.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "secondary_genres.keyword": self.exclude_secondary_genres
          }
        }));
    }

    if !self.include_languages.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "languages.keyword": self.include_languages
        }
      }));
    }

    if !self.exclude_languages.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "languages.keyword": self.exclude_languages
          }
        }));
    }

    if !self.include_descriptors.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "descriptors.keyword": self.include_descriptors
        }
      }));
    }

    if !self.exclude_descriptors.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "descriptors.keyword": self.exclude_descriptors
          }
        }));
    }

    if let Some(min_primary_genre_count) = self.min_primary_genre_count {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "range": {
          "primary_genre_count": {
            "gte": min_primary_genre_count
          }
        }
      }));
    }

    if let Some(min_secondary_genre_count) = self.min_secondary_genre_count {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "range": {
          "secondary_genre_count": {
            "gte": min_secondary_genre_count
          }
        }
      }));
    }

    if let Some(min_descriptor_count) = self.min_descriptor_count {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "range": {
          "descriptor_count": {
            "gte": min_descriptor_count
          }
        }
      }));
    }

    if let Some(min_release_year) = self.min_release_year {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "range": {
          "release_year": {
            "gte": min_release_year
          }
        }
      }));
    }

    if let Some(max_release_year) = self.max_release_year {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "range": {
          "release_year": {
            "lte": max_release_year
          }
        }
      }));
    }

    if let Some(include_duplicates) = self.include_duplicates {
      if include_duplicates {
        query["bool"]["must"].as_array_mut().unwrap().push(json!({
          "exists": {
            "field": "duplicate_of"
          }
        }));
      }
    }

    if query["bool"]["must"].as_array().unwrap().is_empty()
      && query["bool"]["must_not"].as_array().unwrap().is_empty()
    {
      query = json!({
          "match_all": {}
      });
    }
    query
  }
}

impl From<ElasticsearchResult<AlbumReadModel>> for AlbumSearchResult {
  fn from(result: ElasticsearchResult<AlbumReadModel>) -> Self {
    Self {
      albums: result.results.into_iter().map(|item| item.item).collect(),
      total: result.total,
    }
  }
}

pub struct EsAlbumSearchIndex {
  index: ElasticsearchIndex,
}

const INDEX_NAME: &str = "albums";

impl EsAlbumSearchIndex {
  pub fn new(elasticsearch_client: Arc<Elasticsearch>) -> Self {
    Self {
      index: ElasticsearchIndex::new(elasticsearch_client, INDEX_NAME.to_string()),
    }
  }

  pub async fn setup_index(&self) -> Result<()> {
    self.index.setup().await?;
    Ok(())
  }
}

#[async_trait]
impl AlbumSearchIndex for EsAlbumSearchIndex {
  async fn get_embedding_keys(&self) -> Result<Vec<String>> {
    let fields = self.index.list_fields().await?;
    Ok(
      fields
        .into_iter()
        .filter(|field| field.starts_with("embedding_vector_"))
        .map(|field| ElasticsearchIndex::embedding_key_from_field(&field))
        .collect(),
    )
  }

  async fn put_many(&self, albums: Vec<AlbumReadModel>) -> Result<()> {
    self
      .index
      .put_many(
        albums
          .into_iter()
          .map(|album| {
            (
              album.file_name.to_string(),
              json!(Into::<EsAlbumReadModel>::into(album)),
            )
          })
          .collect::<Vec<(String, Value)>>(),
      )
      .await
  }

  async fn put(&self, album: AlbumReadModel) -> Result<()> {
    self.put_many(vec![album]).await
  }

  async fn delete(&self, file_name: &FileName) -> Result<()> {
    self.index.delete(file_name.to_string()).await
  }

  async fn search(
    &self,
    query: &AlbumSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<AlbumSearchResult> {
    let result = self
      .index
      .search(
        json!({
          "_source": {
            "exclude": [ElasticsearchIndex::embedding_field_wildcard()]
          },
          "query": query.to_es_query(),
        }),
        pagination,
      )
      .await?;
    Ok(result.into())
  }

  async fn embedding_similarity_search(
    &self,
    query: &AlbumEmbeddingSimilarirtySearchQuery,
  ) -> Result<Vec<(AlbumReadModel, f32)>> {
    let result = self
      .index
      .search(
        json!({
          "_source": {
            "exclude": [ElasticsearchIndex::embedding_field_wildcard()]
          },
          "knn": {
            "k": query.limit,
            "field": ElasticsearchIndex::embedding_field_key(&query.embedding_key),
            "query_vector": query.embedding,
            "filter": query.filters.to_es_query(),
          }
        }),
        Some(&SearchPagination {
          limit: Some(query.limit),
          ..Default::default()
        }),
      )
      .await?;
    Ok(
      result
        .results
        .into_iter()
        .map(|item| (item.item, item.score))
        .collect(),
    )
  }

  async fn put_many_embeddings(&self, docs: Vec<EmbeddingDocument>) -> Result<()> {
    self
      .index
      .put_many(
        docs
          .into_iter()
          .map(|doc| {
            (
              doc.file_name.to_string(),
              json!({
                ElasticsearchIndex::embedding_field_key(&doc.key): doc.embedding
              }),
            )
          })
          .collect::<Vec<(String, Value)>>(),
      )
      .await?;
    Ok(())
  }

  async fn put_embedding(&self, embedding: EmbeddingDocument) -> Result<()> {
    self.put_many_embeddings(vec![embedding]).await
  }

  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    self
      .index
      .find(
        file_name.to_string(),
        None,
        Some(vec![ElasticsearchIndex::embedding_field_wildcard()]),
      )
      .await
  }

  async fn find_many_embeddings(
    &self,
    file_names: Vec<FileName>,
    key: &str,
  ) -> Result<Vec<EmbeddingDocument>> {
    let key = ElasticsearchIndex::embedding_field_key(key);
    Ok(
      self
        .index
        .find_many::<HashMap<String, Vec<f32>>>(
          file_names
            .into_iter()
            .map(|file_name| file_name.to_string())
            .collect(),
          Some(vec![key.clone()]),
          None,
        )
        .await?
        .into_iter()
        .filter_map(|(doc_id, mut doc)| {
          doc.remove(&key).and_then(|embedding| {
            FileName::try_from(doc_id)
              .inspect_err(|err| error!("Failed to parse file name: {:?}", err))
              .ok()
              .map(|file_name| EmbeddingDocument {
                file_name,
                key: key.clone(),
                embedding,
              })
          })
        })
        .collect(),
    )
  }

  async fn find_embedding(
    &self,
    file_name: &FileName,
    key: &str,
  ) -> Result<Option<EmbeddingDocument>> {
    Ok(
      self
        .find_many_embeddings(vec![file_name.clone()], key)
        .await?
        .into_iter()
        .next(),
    )
  }

  async fn get_embeddings(&self, file_name: &FileName) -> Result<Vec<EmbeddingDocument>> {
    Ok(
      self
        .index
        .find::<HashMap<String, Vec<f32>>>(
          file_name.to_string(),
          None,
          Some(vec![ElasticsearchIndex::embedding_field_wildcard()]),
        )
        .await?
        .map(|doc| {
          doc
            .into_iter()
            .map(|(field, embedding)| EmbeddingDocument {
              file_name: file_name.clone(),
              key: ElasticsearchIndex::embedding_key_from_field(&field),
              embedding,
            })
            .collect()
        })
        .unwrap_or_default(),
    )
  }

  async fn delete_embedding(&self, file_name: &FileName, key: &str) -> Result<()> {
    self
      .index
      .delete_field(
        file_name.to_string(),
        &ElasticsearchIndex::embedding_field_key(key),
      )
      .await
  }
}
