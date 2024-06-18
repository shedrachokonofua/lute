use crate::{
  files::file_metadata::file_name::{FileName, ListRootFileName},
  helpers::document_store::{DocumentFilter, DocumentStore},
  parser::parsed_file_data::ParsedListSegment,
};
use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

const COLLECTION: &str = "list_lookup";

#[derive(Serialize, Deserialize, Clone)]
pub struct ListSegmentDocument {
  pub file_name: FileName,
  pub root_file_name: ListRootFileName,
  pub other_segments: Vec<FileName>,
  pub albums: Vec<FileName>,
}

impl ListSegmentDocument {
  pub fn try_from_parsed_list_segment(
    file_name: FileName,
    data: ParsedListSegment,
  ) -> Result<Self> {
    Ok(Self {
      root_file_name: ListRootFileName::try_from(file_name.clone())?,
      file_name,
      other_segments: data.other_segments,
      albums: data.albums,
    })
  }
}

pub struct ListSegmentRepository {
  doc_store: Arc<DocumentStore>,
}

impl ListSegmentRepository {
  pub fn new(doc_store: Arc<DocumentStore>) -> Self {
    Self { doc_store }
  }

  pub async fn put_many(&self, updates: Vec<ListSegmentDocument>) -> Result<()> {
    self
      .doc_store
      .put_many(
        COLLECTION,
        updates
          .into_iter()
          .map(|d| (d.file_name.to_string(), d, None))
          .collect(),
      )
      .await
  }

  pub async fn find_many_by_root(
    &self,
    root_file_name: &ListRootFileName,
  ) -> Result<Vec<ListSegmentDocument>> {
    self
      .doc_store
      .find_many(
        COLLECTION,
        DocumentFilter::new()
          .condition("root_file_name", "=", root_file_name.to_string())
          .build(),
        None,
      )
      .await
      .map(|d| d.documents.into_iter().map(|d| d.document).collect())
  }
}
