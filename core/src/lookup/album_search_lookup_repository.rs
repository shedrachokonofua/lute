use super::album_search_lookup::{
  AlbumSearchLookup, AlbumSearchLookupDiscriminants, AlbumSearchLookupQuery,
};
use crate::{
  files::file_metadata::file_name::FileName,
  helpers::document_store::{DocumentIndexReadCursorBuilder, DocumentStore},
};
use anyhow::{anyhow, Result};
use std::{collections::HashMap, sync::Arc};
use strum::VariantArray;

const COLLECTION: &str = "album_search_lookup";

pub struct AggregatedStatus {
  pub status: String,
  pub count: u32,
}

pub struct AlbumSearchLookupRepository {
  pub doc_store: Arc<DocumentStore>,
}

impl AlbumSearchLookupRepository {
  pub fn new(doc_store: Arc<DocumentStore>) -> Self {
    Self { doc_store }
  }

  pub async fn find(&self, query: &AlbumSearchLookupQuery) -> Result<Option<AlbumSearchLookup>> {
    self
      .doc_store
      .find::<AlbumSearchLookup>(COLLECTION, &query.to_encoded_string())
      .await
      .map(|d| d.map(|d| d.document))
  }

  pub async fn find_many(
    &self,
    queries: Vec<&AlbumSearchLookupQuery>,
  ) -> Result<HashMap<String, AlbumSearchLookup>> {
    let keys = queries
      .into_iter()
      .map(|q| q.to_encoded_string())
      .collect::<Vec<_>>();
    self
      .doc_store
      .find_many::<AlbumSearchLookup>(COLLECTION, keys)
      .await?
      .into_iter()
      .map(|(k, v)| Ok((k, v.document)))
      .collect()
  }

  pub async fn find_many_by_album_file_name(
    &self,
    file_name: &FileName,
  ) -> Result<Vec<AlbumSearchLookup>> {
    self
      .doc_store
      .read_index::<AlbumSearchLookup>(
        COLLECTION,
        "parsed_album_search_result.file_name",
        DocumentIndexReadCursorBuilder::default()
          .start_key(file_name.to_string())
          .limit(10000)
          .build()?,
      )
      .await
      .map(|d| d.documents.into_iter().map(|d| d.document).collect())
  }

  pub async fn get(&self, query: &AlbumSearchLookupQuery) -> Result<AlbumSearchLookup> {
    match self.find(query).await? {
      Some(lookup) => Ok(lookup),
      None => Err(anyhow!("Not found")),
    }
  }

  pub async fn put(&self, lookup: &AlbumSearchLookup) -> Result<()> {
    self
      .doc_store
      .put(
        COLLECTION,
        &lookup.query().to_encoded_string(),
        lookup,
        None,
      )
      .await
  }

  pub async fn aggregate_statuses(&self) -> Result<Vec<AggregatedStatus>> {
    let statuses = AlbumSearchLookupDiscriminants::VARIANTS
      .iter()
      .map(|status| status.to_string())
      .collect::<Vec<_>>();
    let counts = self
      .doc_store
      .count_many_by_index_key(COLLECTION, "status", statuses)
      .await?;
    Ok(
      counts
        .into_iter()
        .map(|(status, count)| AggregatedStatus {
          status,
          count: count as u32,
        })
        .collect(),
    )
  }

  pub async fn delete(&self, query: &AlbumSearchLookupQuery) -> Result<()> {
    self
      .doc_store
      .delete(COLLECTION, &query.to_encoded_string())
      .await
  }
}
