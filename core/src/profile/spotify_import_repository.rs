use super::{
  profile::ProfileId, spotify_import_lookup_subscription::SpotifyImportLookupSubscription,
};
use crate::{
  helpers::redisearch::{does_ft_index_exist, escape_tag_value},
  lookup::album_search_lookup::AlbumSearchLookupQuery,
};
use anyhow::{anyhow, Error, Result};
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtSearchOptions, JsonCommands,
    SearchCommands, SetCondition,
  },
};
use std::sync::Arc;
use tracing::{info, warn};

pub struct SpotifyImportRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl TryFrom<Vec<(String, String)>> for SpotifyImportLookupSubscription {
  type Error = Error;

  fn try_from(values: Vec<(String, String)>) -> Result<Self> {
    let json = values.get(0).map(|(_, json)| json).ok_or(anyhow!(
      "invalid SpotifyImportLookupSubscription: missing json"
    ))?;
    let subscription: SpotifyImportLookupSubscription = serde_json::from_str(json)?;
    Ok(subscription)
  }
}

const NAMESPACE: &str = "profile_spotify_import";
const INDEX_NAME: &str = "profile_spotify_import_idx";

impl SpotifyImportRepository {
  fn lookup_subscriptions_key(
    &self,
    album_search_lookup_encoded_query: String,
    profile_id: &ProfileId,
  ) -> String {
    format!(
      "{}:{}:{}",
      NAMESPACE,
      album_search_lookup_encoded_query,
      profile_id.to_string()
    )
  }

  pub async fn setup_index(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;

    if !does_ft_index_exist(&connection, &INDEX_NAME.to_string()).await {
      info!("Creating new search index: {}", INDEX_NAME);
      connection
        .ft_create(
          INDEX_NAME,
          FtCreateOptions::default()
            .on(FtIndexDataType::Json)
            .prefix(format!("{}:", NAMESPACE)),
          [
            FtFieldSchema::identifier("$.album_search_lookup_encoded_query")
              .as_attribute("album_search_lookup_encoded_query")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.profile_id")
              .as_attribute("profile_id")
              .field_type(FtFieldType::Tag),
          ],
        )
        .await?;
    }

    Ok(())
  }

  async fn find_subscriptions(
    &self,
    key: String,
    value: String,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    let connection = self.redis_connection_pool.get().await?;
    let search_result = connection
      .ft_search(
        INDEX_NAME,
        format!("@{}:{{ {} }}", key, escape_tag_value(&value)),
        FtSearchOptions::default().limit(0, 10000),
      )
      .await?;

    let result = search_result
      .results
      .into_iter()
      .filter_map(|r| match r.values.try_into() {
        Ok(subscription) => Some(subscription),
        Err(e) => {
          warn!(
            "Failed to deserialize SpotifyImportLookupSubscription: {}",
            e
          );
          None
        }
      })
      .collect::<Vec<_>>();

    Ok(result)
  }

  pub async fn find_subscriptions_by_query(
    &self,
    album_search_lookup_query: &AlbumSearchLookupQuery,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    self
      .find_subscriptions(
        "album_search_lookup_encoded_query".to_string(),
        album_search_lookup_query.to_encoded_string(),
      )
      .await
  }

  pub async fn find_subscriptions_by_profile_id(
    &self,
    profile_id: &ProfileId,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    self
      .find_subscriptions("profile_id".to_string(), profile_id.to_string())
      .await
  }

  pub async fn put_subscription(
    &self,
    album_search_lookup_query: &AlbumSearchLookupQuery,
    profile_id: &ProfileId,
    factor: u32,
  ) -> Result<()> {
    let subscription = SpotifyImportLookupSubscription {
      album_search_lookup_encoded_query: album_search_lookup_query.to_encoded_string(),
      album_search_lookup_query: album_search_lookup_query.clone(),
      profile_id: profile_id.clone(),
      factor,
    };
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        self.lookup_subscriptions_key(album_search_lookup_query.to_encoded_string(), profile_id),
        "$",
        serde_json::to_string(&subscription)?,
        SetCondition::default(),
      )
      .await?;

    Ok(())
  }

  pub async fn delete_subscription(
    &self,
    profile_id: &ProfileId,
    album_search_lookup_query: &AlbumSearchLookupQuery,
  ) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .json_del(
        self.lookup_subscriptions_key(album_search_lookup_query.to_encoded_string(), profile_id),
        "$",
      )
      .await?;

    Ok(())
  }
}
