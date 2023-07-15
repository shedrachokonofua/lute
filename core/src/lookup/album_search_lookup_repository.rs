use crate::helpers::db::does_ft_index_exist;

use super::album_search_lookup::{AlbumSearchLookup, AlbumSearchLookupQuery};
use anyhow::{anyhow, Result};
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtAggregateOptions, FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtReducer,
    HashCommands, SearchCommands,
  },
};
use std::{collections::HashMap, sync::Arc};
use tracing::info;

const NAMESPACE: &str = "lookup:album_search";
const INDEX_NAME: &str = "lookup:album_search_idx";

fn key(query: &AlbumSearchLookupQuery) -> String {
  format!("{}:{}", NAMESPACE, query.to_encoded_string())
}

pub struct AggregatedStatus {
  pub status: String,
  pub count: u32,
}

impl From<Vec<(String, String)>> for AggregatedStatus {
  fn from(val: Vec<(String, String)>) -> Self {
    let mut status = None;
    let mut count = None;

    for (key, value) in val {
      match key.as_str() {
        "status" => status = Some(value),
        "count" => count = Some(value.parse().expect("invalid count")),
        _ => {}
      }
    }

    Self {
      status: status.expect("status not found"),
      count: count.expect("count not found"),
    }
  }
}

pub struct AlbumSearchLookupRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl AlbumSearchLookupRepository {
  pub async fn find(&self, query: &AlbumSearchLookupQuery) -> Result<Option<AlbumSearchLookup>> {
    let res: HashMap<String, String> = self
      .redis_connection_pool
      .get()
      .await?
      .hgetall(key(query))
      .await?;

    match res.is_empty() {
      true => Ok(None),
      false => Ok(Some(AlbumSearchLookup::try_from(res)?)),
    }
  }

  pub async fn get(&self, query: &AlbumSearchLookupQuery) -> Result<AlbumSearchLookup> {
    match self.find(query).await? {
      Some(lookup) => Ok(lookup),
      None => Err(anyhow!("Not found")),
    }
  }

  pub async fn put(&self, lookup: &AlbumSearchLookup) -> Result<()> {
    let key = key(&lookup.query());
    let map: HashMap<String, String> = (*lookup).clone().into();
    self
      .redis_connection_pool
      .get()
      .await?
      .hset(key, map)
      .await?;
    Ok(())
  }

  pub async fn aggregate_statuses(&self) -> Result<Vec<AggregatedStatus>> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_aggregate(
        INDEX_NAME,
        "*",
        FtAggregateOptions::default().groupby("@status", FtReducer::count().as_name("count")),
      )
      .await?;
    let aggregates = result
      .results
      .iter()
      .map(|r| AggregatedStatus::from(r.to_owned()))
      .collect::<Vec<_>>();

    Ok(aggregates)
  }

  pub async fn setup_index(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    if !does_ft_index_exist(&connection, INDEX_NAME).await {
      connection
        .ft_create(
          INDEX_NAME,
          FtCreateOptions::default()
            .on(FtIndexDataType::Hash)
            .prefix(format!("{}:", NAMESPACE)),
          [
            FtFieldSchema::identifier("status").field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("album_search_file_name").field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("album_file_name").field_type(FtFieldType::Tag),
          ],
        )
        .await?;
    }
    Ok(())
  }
}
