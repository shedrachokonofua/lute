use super::{
  profile::ProfileId, spotify_import_lookup_subscription::SpotifyImportLookupSubscription,
};
use crate::{
  helpers::document_store::{DocumentIndexReadCursorBuilder, DocumentStore},
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
    key: String,
    value: String,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    let mut docs = vec![];
    let mut next_id_cursor = None;
    loop {
      let mut cursor = DocumentIndexReadCursorBuilder::default();
      cursor.start_key(value.clone()).limit(500);
      if let Some(next_id_cursor) = next_id_cursor {
        cursor.id_cursor(next_id_cursor);
      }
      let cursor = cursor.build()?;
      let res = self
        .doc_store
        .read_index::<SpotifyImportLookupSubscription>(COLLECTION, &key, cursor)
        .await?;
      docs.extend(res.documents);
      if res.next_id_cursor.is_none() {
        break;
      }
      next_id_cursor = res.next_id_cursor;
    }

    Ok(docs.into_iter().map(|doc| doc.document).collect())
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
