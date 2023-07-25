use super::album_search_lookup::{AlbumSearchLookup, AlbumSearchLookupQuery};
use crate::{
  files::file_metadata::file_name::FileName,
  helpers::redisearch::{does_ft_index_exist, escape_tag_value},
};
use anyhow::{anyhow, Result};
use rustis::{
  bb8::Pool,
  client::{BatchPreparedCommand, PooledClientManager},
  commands::{
    FtAggregateOptions, FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtReducer,
    FtSearchOptions, HashCommands, SearchCommands,
  },
};
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

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

  pub async fn find_many(
    &self,
    queries: Vec<&AlbumSearchLookupQuery>,
  ) -> Result<Vec<Option<AlbumSearchLookup>>> {
    let connection = self.redis_connection_pool.get().await?;
    let mut pipeline = connection.create_pipeline();
    for query in queries {
      pipeline
        .hgetall::<String, _, _, HashMap<String, String>>(key(query))
        .queue();
    }
    let results: Vec<HashMap<String, String>> = pipeline.execute().await?;
    results
      .into_iter()
      .map(|res| match res.is_empty() {
        true => Ok(None),
        false => Ok(Some(AlbumSearchLookup::try_from(res)?)),
      })
      .collect::<Result<Vec<_>>>()
  }

  pub async fn find_many_by_album_file_name(
    &self,
    file_name: &FileName,
  ) -> Result<Vec<AlbumSearchLookup>> {
    let connection = self.redis_connection_pool.get().await?;
    let search_result = connection
      .ft_search(
        INDEX_NAME,
        format!(
          "@album_file_name:{{ {} }}",
          escape_tag_value(&file_name.to_string())
        ),
        FtSearchOptions::default().limit(0, 10000),
      )
      .await?;
    let result = search_result
      .results
      .iter()
      .map(|r| {
        let lookup: Result<AlbumSearchLookup> = r
          .values
          .clone()
          .into_iter()
          .collect::<HashMap<_, _>>()
          .try_into();
        lookup
      })
      .filter_map(|r| match r {
        Ok(lookup) => Some(lookup),
        Err(err) => {
          warn!("Failed to deserialize AlbumSearchLookup: {}", err);
          None
        }
      })
      .collect::<Vec<_>>();

    Ok(result)
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
}
