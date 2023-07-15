use super::{
  profile::ProfileId, spotify_import_lookup_subscription::SpotifyImportLookupSubscription,
};
use crate::{
  helpers::db::does_ft_index_exist, lookup::album_search_lookup::AlbumSearchLookupQuery,
};
use anyhow::Result;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtSearchOptions, JsonCommands,
    SearchCommands, SetCondition,
  },
};
use std::{collections::HashMap, sync::Arc};
use tracing::info;

pub struct SpotifyImportRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl From<Vec<(String, String)>> for SpotifyImportLookupSubscription {
  fn from(values: Vec<(String, String)>) -> Self {
    let mut values = values.into_iter().collect::<HashMap<_, _>>();

    let album_search_lookup_encoded_query = values
      .remove("album_search_lookup_encoded_query")
      .expect("album_search_lookup_encoded_query not found");
    let profile_id =
      ProfileId::try_from(values.remove("profile_id").expect("profile_id not found"))
        .expect("invalid profile_id");
    let factor = values
      .remove("factor")
      .expect("factor not found")
      .parse::<u32>()
      .unwrap();
    let album_search_lookup_query: AlbumSearchLookupQuery = serde_json::from_str(
      &values
        .remove("album_search_lookup_query")
        .expect("album_search_lookup_query not found"),
    )
    .expect("invalid album_search_lookup_query");

    Self {
      album_search_lookup_encoded_query,
      profile_id,
      factor,
      album_search_lookup_query,
    }
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

    if !does_ft_index_exist(&connection, INDEX_NAME).await {
      info!("Creating new search index: {}", INDEX_NAME);
      connection
        .ft_create(
          INDEX_NAME,
          FtCreateOptions::default()
            .on(FtIndexDataType::Json)
            .prefix(format!("{}:", NAMESPACE)),
          [
            FtFieldSchema::identifier("$.album_search_lookup_encoded_query")
              .field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("$.profile_id").field_type(FtFieldType::Tag),
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
        format!("@{}:'{}'", key, value),
        FtSearchOptions::default(),
      )
      .await?;

    let result = search_result
      .results
      .iter()
      .map(|result| {
        let subscription: SpotifyImportLookupSubscription = result.values.clone().into();
        subscription
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
    profile_id: ProfileId,
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

  pub async fn remove_subscription(
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
