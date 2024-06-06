use super::artist_read_model::ArtistOverview;
use crate::{
  files::file_metadata::file_name::FileName, helpers::redisearch::SearchPagination, proto,
};
use anyhow::{anyhow, Result};
use derive_builder::Builder;
use elasticsearch::http::request::JsonBody;
use elasticsearch::{BulkParts, Elasticsearch, IndexParts, SearchParts};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::{max, min};
use std::sync::Arc;
use tracing::{info, instrument};

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
          "primary_genres": self.include_primary_genres
        }
      }));
    }
    if !self.include_secondary_genres.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "secondary_genres": self.include_secondary_genres
        }
      }));
    }
    if !self.include_credit_roles.is_empty() {
      query["bool"]["must"].as_array_mut().unwrap().push(json!({
        "terms": {
          "credit_roles": self.include_credit_roles
        }
      }));
    }
    if !self.exclude_file_names.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "file_name": self.exclude_file_names
          }
        }));
    }
    if !self.exclude_primary_genres.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "terms": {
              "primary_genres": self.exclude_primary_genres
          }
        }));
    }
    if !self.exclude_secondary_genres.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "terms": {
              "secondary_genres": self.exclude_secondary_genres
          }
        }));
    }
    if !self.exclude_credit_roles.is_empty() {
      query["bool"]["must_not"]
        .as_array_mut()
        .unwrap()
        .push(json!({
          "terms": {
            "credit_roles": self.exclude_credit_roles
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

const INDEX_NAME: &str = "poc-artists";

pub struct ArtistSearchIndex {
  elasticsearch_client: Arc<Elasticsearch>,
}

impl ArtistSearchIndex {
  pub fn new(elasticsearch_client: Arc<Elasticsearch>) -> Self {
    Self {
      elasticsearch_client,
    }
  }

  pub async fn setup_index(&self) -> Result<()> {
    let res = self
      .elasticsearch_client
      .index(IndexParts::Index(INDEX_NAME))
      .body(json!({
        "settings": {
          "number_of_shards": 1,
          "number_of_replicas": 0,
          "analysis": {
            "analyzer": {
              "ascii_analyzer": {
                "tokenizer": "standard",
                "filter": [
                  "lowercase",
                  "asciifolding"
                ]
              }
            }
          }
        },
        "mappings": {
          "properties": {
            "name": {
              "type": "text",
              "analyzer": "ascii_analyzer"
            },
            "file_name": {
              "type": "keyword"
            },
            "total_rating_count": {
              "type": "integer"
            },
            "min_year": {
              "type": "integer"
            },
            "max_year": {
              "type": "integer"
            },
            "album_count": {
              "type": "integer"
            },
            "credit_roles": {
              "type": "keyword"
            },
            "primary_genres": {
              "type": "keyword"
            },
            "creditted_primary_genres": {
              "type": "keyword"
            },
            "secondary_genres": {
              "type": "keyword"
            },
            "creditted_secondary_genres": {
              "type": "keyword"
            }
          }
        },
      }))
      .send()
      .await?;
    let response_body = res.json::<Value>().await?;
    info!("ElasticSearch index setup: {:?}", response_body);
    Ok(())
  }

  #[instrument(skip_all, fields(artists = artists.len()))]
  pub async fn put_many(&self, artists: Vec<ArtistSearchRecord>) -> Result<()> {
    let count = artists.len();
    let body = artists
      .into_iter()
      .flat_map(|artist| {
        vec![
          json!({
            "index": {
              "_id": artist.file_name
            }
          })
          .into(),
          json!(artist).into(),
        ]
      })
      .collect::<Vec<JsonBody<Value>>>();
    let res = self
      .elasticsearch_client
      .bulk(BulkParts::Index(INDEX_NAME))
      .body(body)
      .send()
      .await?;
    let response_body = res.json::<Value>().await?;
    if response_body["errors"].as_bool().unwrap_or(false) {
      return Err(anyhow!("Failed to put artists: {:?}", response_body));
    }
    let took = response_body["took"].as_i64().unwrap();
    info!("ElasticSearch put {} artists in {}ms", count, took);
    Ok(())
  }

  pub async fn put(&self, artist: ArtistSearchRecord) -> Result<()> {
    self.put_many(vec![artist]).await
  }

  #[instrument(skip(self))]
  pub async fn search(
    &self,
    query: &ArtistSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<ArtistSearchResult> {
    let offset = pagination
      .and_then(|p: &SearchPagination| p.offset)
      .unwrap_or(0) as i64;
    let limit = pagination.and_then(|p| p.limit).unwrap_or(50) as i64;
    let query = query.to_es_query();
    let res = self
      .elasticsearch_client
      .search(SearchParts::Index(&[INDEX_NAME]))
      .from(offset)
      .size(limit)
      .body(json!({
        "query": query,
      }))
      .send()
      .await?;

    let mut response_body = res.json::<Value>().await?;

    let took = response_body["took"].as_i64().unwrap();
    let total = response_body["hits"]["total"]["value"].as_i64().unwrap();
    info!(
      "ElasticSearch returned artists in {}ms, total: {}",
      took, total
    );

    let records = response_body["hits"]["hits"]
      .as_array_mut()
      .unwrap()
      .into_iter()
      .filter_map(|hit| {
        serde_json::from_value::<ArtistSearchRecord>(hit["_source"].take())
          .inspect_err(|e| {
            info!("Failed to parse ArtistSearchRecord: {:?}", e);
          })
          .ok()
      })
      .collect::<Vec<ArtistSearchRecord>>();

    Ok(ArtistSearchResult {
      artists: records,
      total: total as usize,
    })
  }
}
