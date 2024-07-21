use anyhow::{anyhow, Result};
use elasticsearch::{
  http::request::JsonBody, BulkParts, DeleteParts, Elasticsearch, IndexParts, MgetParts,
  SearchParts, UpdateParts,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info, instrument};

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

  pub fn embedding_key_from_field(field: &str) -> String {
    field.replace("embedding_vector_", "")
  }

  pub fn embedding_field_key(key: &str) -> String {
    format!("embedding_vector_{}", key)
  }

  pub fn embedding_field_wildcard() -> String {
    "embedding_vector_*".to_string()
  }

  #[instrument(skip(self))]
  pub async fn setup(&self) -> Result<()> {
    let body = json!({
      "settings": {
        "number_of_shards": 1,
        "number_of_replicas": 0,
        "index.max_result_window": 200000,
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
      .track_total_hits(true)
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
      .filter_map(|hit| {
        serde_json::from_value(hit["_source"].take())
          .inspect_err(|e| {
            error!(
              e = e.to_string(),
              "Failed to deserialize Elasticsearch result item"
            );
          })
          .ok()
          .map(|item| ElasticsearchResultItem {
            id: hit["_id"].as_str().unwrap_or_default().to_string(),
            score: hit["_score"].as_f64().unwrap_or_default() as f32,
            item,
          })
      })
      .collect::<Vec<ElasticsearchResultItem<T>>>();

    Ok(ElasticsearchResult {
      results: records,
      total: total as usize,
    })
  }

  #[instrument(skip_all)]
  pub async fn delete(&self, id: String) -> Result<()> {
    let res = self
      .client
      .delete(DeleteParts::IndexId(self.index_name.as_str(), id.as_str()))
      .send()
      .await?;
    let response_body = res.json::<Value>().await?;
    if response_body["result"].as_str() != Some("deleted") {
      return Err(anyhow!("Failed to delete document: {:?}", response_body));
    }
    Ok(())
  }

  #[instrument(skip_all)]
  pub async fn find_many<T: DeserializeOwned>(
    &self,
    ids: Vec<String>,
    include_fields: Option<Vec<String>>,
    exclude_fields: Option<Vec<String>>,
  ) -> Result<HashMap<String, T>> {
    let res = self
      .client
      .mget(MgetParts::Index(self.index_name.as_str()))
      ._source_includes(
        include_fields
          .unwrap_or_default()
          .iter()
          .map(AsRef::as_ref)
          .collect::<Vec<&str>>()
          .as_slice(),
      )
      ._source_excludes(
        exclude_fields
          .unwrap_or_default()
          .iter()
          .map(AsRef::as_ref)
          .collect::<Vec<&str>>()
          .as_slice(),
      )
      .body(json!({
        "docs": ids.iter().map(|id| json!({"_id": id})).collect::<Vec<Value>>()
      }))
      .send()
      .await?;

    let mut response_body = res.json::<Value>().await?;
    let docs = response_body["docs"]
      .as_array_mut()
      .unwrap()
      .iter_mut()
      .filter_map(|doc| {
        serde_json::from_value(doc["_source"].take())
          .inspect_err(|e| {
            error!(
              e = e.to_string(),
              "Failed to deserialize Elasticsearch result item"
            );
          })
          .ok()
          .map(|item| (doc["_id"].as_str().unwrap_or_default().to_string(), item))
      })
      .collect::<HashMap<String, T>>();

    Ok(docs)
  }

  #[instrument(skip_all)]
  pub async fn find<T: DeserializeOwned>(
    &self,
    id: String,
    include_fields: Option<Vec<String>>,
    exclude_fields: Option<Vec<String>>,
  ) -> Result<Option<T>> {
    self
      .find_many(vec![id.clone()], include_fields, exclude_fields)
      .await
      .map(|mut map| map.remove(&id))
  }

  #[instrument(skip_all)]
  pub async fn delete_field(&self, id: String, field: &str) -> Result<()> {
    let res = self
      .client
      .update(UpdateParts::IndexId(self.index_name.as_str(), id.as_str()))
      .body(json!({
        "script": {
          "source": format!("ctx._source.remove('{}')", field),
          "lang": "painless"
        }
      }))
      .send()
      .await?;
    let response_body = res.json::<Value>().await?;
    if response_body["result"].as_str() != Some("updated") {
      return Err(anyhow!("Failed to delete field: {:?}", response_body));
    }
    Ok(())
  }
}
