use super::artist_read_model::ArtistOverview;
use crate::helpers::elasticsearch_index::{ElasticsearchIndex, ElasticsearchResult};
use crate::helpers::embedding::EmbeddingDocument;
use crate::{
  files::file_metadata::file_name::FileName, helpers::redisearch::SearchPagination, proto,
};
use anyhow::{anyhow, Result};
use derive_builder::Builder;
use elasticsearch::Elasticsearch;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct ArtistEmbeddingSimilarirtySearchQuery {
  pub embedding: Vec<f32>,
  pub embedding_key: String,
  pub filters: ArtistSearchQuery,
  pub limit: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct ArtistSearchRecord {
  pub name: String,
  pub file_name: String,
  pub total_rating_count: u32,
  pub min_year: u32,
  pub max_year: u32,
  pub album_count: u32,
  pub credit_roles: Vec<String>,
  pub primary_genres: Vec<String>,
  pub creditted_primary_genres: Vec<String>,
  pub secondary_genres: Vec<String>,
  pub creditted_secondary_genres: Vec<String>,
}

impl TryFrom<Vec<(String, String)>> for ArtistSearchRecord {
  type Error = anyhow::Error;

  fn try_from(values: Vec<(String, String)>) -> Result<Self> {
    let json = values
      .get(1)
      .map(|(_, json)| json)
      .ok_or(anyhow!("invalid ArtistSearchRecord: missing json"))?;
    let record: ArtistSearchRecord = serde_json::from_str(json)?;
    Ok(record)
  }
}

#[derive(Debug)]
pub struct ArtistSearchResult {
  pub artists: Vec<ArtistSearchRecord>,
  pub total: usize,
}

impl From<ElasticsearchResult<ArtistSearchRecord>> for ArtistSearchResult {
  fn from(val: ElasticsearchResult<ArtistSearchRecord>) -> Self {
    ArtistSearchResult {
      artists: val.results.into_iter().map(|item| item.item).collect(),
      total: val.total,
    }
  }
}

fn get_min_year(left: u32, right: u32) -> u32 {
  if left == 0 {
    right
  } else if right == 0 {
    left
  } else {
    min(left, right)
  }
}

impl From<ArtistOverview> for ArtistSearchRecord {
  fn from(overview: ArtistOverview) -> Self {
    Self {
      name: overview.name.clone(),
      file_name: overview.file_name.to_string(),
      total_rating_count: overview.album_summary.total_rating_count
        + overview.credited_album_summary.total_rating_count,
      min_year: get_min_year(
        overview.album_summary.min_year,
        overview.credited_album_summary.min_year,
      ),
      max_year: max(
        overview.album_summary.max_year,
        overview.credited_album_summary.max_year,
      ),
      album_count: overview.album_summary.album_count,
      credit_roles: overview
        .credit_roles
        .into_iter()
        .map(|item| item.item)
        .collect(),
      primary_genres: overview
        .album_summary
        .primary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
      creditted_primary_genres: overview
        .credited_album_summary
        .primary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
      secondary_genres: overview
        .album_summary
        .secondary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
      creditted_secondary_genres: overview
        .credited_album_summary
        .secondary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
    }
  }
}

#[derive(Default, Builder, Debug)]
#[builder(setter(into), default)]
pub struct ArtistSearchQuery {
  pub text: Option<String>,
  pub exclude_file_names: Vec<FileName>,
  pub include_primary_genres: Vec<String>,
  pub exclude_primary_genres: Vec<String>,
  pub include_secondary_genres: Vec<String>,
  pub exclude_secondary_genres: Vec<String>,
  pub include_credit_roles: Vec<String>,
  pub exclude_credit_roles: Vec<String>,
  pub active_years_range: Option<(u32, u32)>,
  pub min_album_count: Option<u32>,
}

impl TryFrom<proto::ArtistSearchQuery> for ArtistSearchQuery {
  type Error = anyhow::Error;

  fn try_from(value: proto::ArtistSearchQuery) -> Result<Self> {
    Ok(Self {
      text: value.text,
      exclude_file_names: value
        .exclude_file_names
        .into_iter()
        .map(FileName::try_from)
        .collect::<Result<Vec<_>>>()?,
      include_primary_genres: value.include_primary_genres,
      exclude_primary_genres: value.exclude_primary_genres,
      include_secondary_genres: value.include_secondary_genres,
      exclude_secondary_genres: value.exclude_secondary_genres,
      include_credit_roles: value.include_credit_roles,
      exclude_credit_roles: value.exclude_credit_roles,
      active_years_range: value.active_years_range.map(|r| (r.start, r.end)),
      min_album_count: value.min_album_count,
    })
  }
}

impl ArtistSearchQuery {
  pub fn to_es_query(&self) -> Value {
    let mut query = json!({
      "bool": {
        "must": [],
        "must_not": []
      }
    });
    if let Some(text) = &self.text {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "match": {
          "name": {
            "query": text,
            "fuzziness": "AUTO"
          }
        }
      }));
    }
    if let Some((start_year, end_year)) = self.active_years_range {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "range": {
          "min_year": {
            "gte": start_year
          }
        }
      }));
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "range": {
          "max_year": {
            "lte": end_year
          }
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
    if !self.include_secondary_genres.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "secondary_genres.keyword": self.include_secondary_genres
        }
      }));
    }
    if !self.include_credit_roles.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "credit_roles.keyword": self.include_credit_roles
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
    if !self.exclude_credit_roles.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "credit_roles.keyword": self.exclude_credit_roles
          }
        }));
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

const INDEX_NAME: &str = "artists";

pub struct ArtistSearchIndex {
  index: ElasticsearchIndex,
}

impl ArtistSearchIndex {
  pub fn new(elasticsearch_client: Arc<Elasticsearch>) -> Self {
    Self {
      index: ElasticsearchIndex::new(elasticsearch_client, INDEX_NAME.to_string()),
    }
  }

  pub async fn setup_index(&self) -> Result<()> {
    self.index.setup().await?;
    Ok(())
  }

  pub async fn put_many_embeddings(&self, docs: Vec<EmbeddingDocument>) -> Result<()> {
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

  pub async fn put_many(&self, artists: Vec<ArtistSearchRecord>) -> Result<()> {
    self
      .index
      .put_many(
        artists
          .into_iter()
          .map(|artist| (artist.file_name.clone(), json!(artist)))
          .collect::<Vec<(String, Value)>>(),
      )
      .await?;
    Ok(())
  }

  pub async fn put(&self, artist: ArtistSearchRecord) -> Result<()> {
    self.put_many(vec![artist]).await
  }

  pub async fn search(
    &self,
    query: &ArtistSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<ArtistSearchResult> {
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

  pub async fn find_embedding(
    &self,
    file_name: &FileName,
    key: &str,
  ) -> Result<Option<EmbeddingDocument>> {
    let key = ElasticsearchIndex::embedding_field_key(key);
    Ok(
      self
        .index
        .find::<HashMap<String, Vec<f32>>>(file_name.to_string(), Some(vec![key.clone()]), None)
        .await?
        .and_then(|mut doc| doc.remove(&key))
        .map(|embedding| EmbeddingDocument {
          file_name: file_name.clone(),
          key,
          embedding,
        }),
    )
  }

  pub async fn embedding_similarity_search(
    &self,
    query: &ArtistEmbeddingSimilarirtySearchQuery,
  ) -> Result<Vec<(ArtistSearchRecord, f32)>> {
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

  pub async fn delete_embedding(&self, file_name: &FileName, key: &str) -> Result<()> {
    self
      .index
      .delete_field(
        file_name.to_string(),
        &ElasticsearchIndex::embedding_field_key(key),
      )
      .await
  }
}
