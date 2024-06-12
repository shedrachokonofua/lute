use super::{
  profile::ProfileId, spotify_import_lookup_subscription::SpotifyImportLookupSubscription,
};
use crate::{
  helpers::document_store::{DocumentFilter, DocumentStore},
  lookup::album_search_lookup::AlbumSearchLookupQuery,
};
use anyhow::Result;
use std::sync::Arc;

pub struct SpotifyImportRepository {
  doc_store: Arc<DocumentStore>,
}

const COLLECTION: &str = "profile_spotify_import";

impl SpotifyImportRepository {
  pub fn new(doc_store: Arc<DocumentStore>) -> Self {
    Self { doc_store }
  }

  fn lookup_subscriptions_key(
    &self,
    album_search_lookup_encoded_query: String,
    profile_id: &ProfileId,
  ) -> String {
    format!(
      "{}:{}",
      album_search_lookup_encoded_query,
      profile_id.to_string()
    )
  }

  async fn find_subscriptions(
    &self,
    key: &str,
    value: String,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    let docs = self
      .doc_store
      .find_many(
        COLLECTION,
        DocumentFilter::new().condition(key, "=", value).build(),
        None,
      )
      .await?
      .documents
      .into_iter()
      .map(|d| d.document)
      .collect();
    Ok(docs)
  }

  pub async fn find_subscriptions_by_query(
    &self,
    album_search_lookup_query: &AlbumSearchLookupQuery,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    self
      .find_subscriptions(
        "album_search_lookup_encoded_query",
        album_search_lookup_query.to_encoded_string(),
      )
      .await
  }

  pub async fn find_subscriptions_by_profile_id(
    &self,
    profile_id: &ProfileId,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    self
      .find_subscriptions("profile_id", profile_id.to_string())
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
      .doc_store
      .put(
        COLLECTION,
        &self.lookup_subscriptions_key(album_search_lookup_query.to_encoded_string(), profile_id),
        subscription,
        None,
      )
      .await
  }

  pub async fn delete_subscription(
    &self,
    profile_id: &ProfileId,
    album_search_lookup_query: &AlbumSearchLookupQuery,
  ) -> Result<()> {
    self
      .doc_store
      .delete(
        COLLECTION,
        &self.lookup_subscriptions_key(album_search_lookup_query.to_encoded_string(), profile_id),
      )
      .await
  }
}
