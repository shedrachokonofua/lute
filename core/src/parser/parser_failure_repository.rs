use crate::{
  files::file_metadata::{file_name::FileName, page_type::PageType},
  helpers::document_store::{DocumentFilter, DocumentStore},
};
use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserFailure {
  pub file_name: FileName,
  pub error: String,
  pub last_attempted_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserFailureDocument {
  pub file_name: FileName,
  pub error: String,
  pub last_attempted_at: NaiveDateTime,
  pub page_type: PageType,
}

impl From<ParserFailure> for ParserFailureDocument {
  fn from(parser_failure: ParserFailure) -> Self {
    Self {
      file_name: parser_failure.file_name.clone(),
      error: parser_failure.error,
      last_attempted_at: parser_failure.last_attempted_at,
      page_type: parser_failure.file_name.page_type(),
    }
  }
}

impl From<ParserFailureDocument> for ParserFailure {
  fn from(doc: ParserFailureDocument) -> Self {
    Self {
      file_name: doc.file_name,
      error: doc.error,
      last_attempted_at: doc.last_attempted_at,
    }
  }
}

pub struct AggregatedError {
  pub error: String,
  pub count: u64,
}

const COLLECTION: &str = "parser_failure";

pub struct ParserFailureRepository {
  pub doc_store: Arc<DocumentStore>,
}

impl ParserFailureRepository {
  pub fn new(doc_store: Arc<DocumentStore>) -> Self {
    Self { doc_store }
  }

  pub async fn put(&self, parser_failure: ParserFailure) -> Result<()> {
    self
      .doc_store
      .put::<ParserFailureDocument>(
        COLLECTION,
        &parser_failure.file_name.to_string(),
        parser_failure.into(),
        None,
      )
      .await
  }

  pub async fn remove(&self, file_name: &FileName) -> Result<()> {
    self
      .doc_store
      .delete(COLLECTION, &file_name.to_string())
      .await
  }

  pub async fn find_many(&self, error: Option<String>) -> Result<Vec<ParserFailure>> {
    let mut filter = DocumentFilter::new();
    if error.is_some() {
      filter.condition("error", "=", error);
    }
    let docs = self
      .doc_store
      .find_many::<ParserFailureDocument>(COLLECTION, filter, None)
      .await?
      .documents
      .into_iter()
      .map(|d| d.document.into())
      .collect::<Vec<_>>();
    Ok(docs)
  }

  pub async fn aggregate_errors(
    &self,
    page_type: Option<PageType>,
  ) -> Result<Vec<AggregatedError>> {
    let mut filter: Option<DocumentFilter> = None;
    if page_type.is_some() {
      filter = Some(
        DocumentFilter::new()
          .condition("page_type", "=", page_type.unwrap().to_string())
          .build(),
      );
    }
    let counts = self
      .doc_store
      .count_each_field_value(COLLECTION, "error", filter)
      .await?;
    let aggregates = counts
      .into_iter()
      .map(|(error, count)| AggregatedError {
        error,
        count: count as u64,
      })
      .collect::<Vec<_>>();

    Ok(aggregates)
  }
}
