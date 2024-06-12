use anyhow::{anyhow, Result};
use elasticsearch::{http::request::JsonBody, BulkParts, Elasticsearch, IndexParts, SearchParts};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, instrument};

use super::redisearch::SearchPagination;

pub struct ElasticsearchIndex {
  pub client: Arc<Elasticsearch>,
  pub index_name: String,
}

pub struct ElasticsearchResultItem<T: DeserializeOwned> {
  pub id: String,
  pub score: f32,
  pub item: T,
}

pub struct ElasticsearchResult<T: DeserializeOwned> {
  pub total: usize,
  pub results: Vec<ElasticsearchResultItem<T>>,
}

impl ElasticsearchIndex {
  pub fn new(client: Arc<Elasticsearch>, index_name: String) -> Self {
    Self { client, index_name }
  }

  #[instrument(skip(self))]
  pub async fn setup(&self) -> Result<()> {
    let body = json!({
      "settings": {
        "number_of_shards": 1,
        "number_of_replicas": 0,
        "max_result_window": 200000,
      },
      "mappings": {
        "dynamic_templates": [
          {
            "file_name_fields": {
              "match_mapping_type": "string",
              "match": "*file_name*",
              "mapping": {
                "type": "keyword"
              }
            }
          },
          {
            "file_name_subfields": {
              "match_mapping_type": "string",
              "path_match": "*.*file_name*",
              "mapping": {
                "type": "keyword"
              }
            }
          },
          {
            "embedding_fields": {
              "match_mapping_type": "float",
              "match": "embedding_vector_*",
              "mapping": {
                "type": "dense_vector",
                "index": true,
                "index_options": {
                  "type": "int8_hnsw"
                }
              }
            }
          }
        ]
      },
    });

    let response = self
      .client
      .index(IndexParts::Index(self.index_name.as_str()))
      .body(body)
      .send()
      .await?;
    let response = response.json::<serde_json::Value>().await?;
    info!("Elasticsearch index created: {:?}", response);

    Ok(())
  }

  #[instrument(skip_all, fields(count = docs.len()))]
  pub async fn put_many(&self, docs: Vec<(String, Value)>) -> Result<()> {
    let count = docs.len();
    let body = docs
      .into_iter()
      .flat_map(|(id, doc)| {
        vec![
          json!({"update": {"_id": id}}).into(),
          json!({"doc": doc, "doc_as_upsert": true}).into(),
        ]
      })
      .collect::<Vec<JsonBody<Value>>>();
    let res = self
      .client
      .bulk(BulkParts::Index(self.index_name.as_str()))
      .body(body)
      .send()
      .await?;
    let response_body = res.json::<Value>().await?;
    if response_body["errors"].as_bool().unwrap_or(false) {
      return Err(anyhow!("Failed to put documents: {:?}", response_body));
    }
    let took = response_body["took"].as_i64().unwrap();
    info!("ElasticSearch put {} documents in {}ms", count, took);
    Ok(())
  }

  #[instrument(skip_all)]
  pub async fn search<T: DeserializeOwned>(
    &self,
    body: Value,
    pagination: Option<&SearchPagination>,
  ) -> Result<ElasticsearchResult<T>> {
    let offset = pagination.and_then(|p| p.offset).unwrap_or(0) as i64;
    let limit = pagination.and_then(|p| p.limit).unwrap_or(50) as i64;
    let res = self
      .client
      .search(SearchParts::Index(&[self.index_name.as_str()]))
      .from(offset)
      .size(limit)
      .body(body)
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
      .iter_mut()
      .map(|hit| {
        Ok(ElasticsearchResultItem {
          id: hit["_id"].as_str().unwrap_or_default().to_string(),
          score: hit["_score"].as_f64().unwrap_or_default() as f32,
          item: serde_json::from_value(hit["_source"].take())?,
        })
      })
      .collect::<Result<Vec<ElasticsearchResultItem<T>>>>()?;

    Ok(ElasticsearchResult {
      results: records,
      total: total as usize,
    })
  }
}
