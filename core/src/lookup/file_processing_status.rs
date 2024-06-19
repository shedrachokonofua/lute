use crate::{
  files::file_metadata::file_name::FileName, helpers::key_value_store::KeyValueStore, proto,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

#[derive(
  Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, strum::Display, Hash,
)]
pub enum FileProcessingStatus {
  CrawlEnqueued = 1,
  CrawlFailed = 2,
  FileSaved = 3,
  FileParsed = 4,
  FileParseFailed = 5,
  ReadModelUpdated = 6,
}

impl From<FileProcessingStatus> for proto::FileProcessingStatus {
  fn from(status: FileProcessingStatus) -> Self {
    match status {
      FileProcessingStatus::CrawlEnqueued => proto::FileProcessingStatus::CrawlEnqueued,
      FileProcessingStatus::CrawlFailed => proto::FileProcessingStatus::CrawlFailed,
      FileProcessingStatus::FileSaved => proto::FileProcessingStatus::FileSaved,
      FileProcessingStatus::FileParsed => proto::FileProcessingStatus::FileParsed,
      FileProcessingStatus::FileParseFailed => proto::FileProcessingStatus::FileParseFailed,
      FileProcessingStatus::ReadModelUpdated => proto::FileProcessingStatus::ReadModelUpdated,
    }
  }
}

impl FileProcessingStatus {
  fn is_error(&self) -> bool {
    matches!(
      self,
      FileProcessingStatus::CrawlFailed | FileProcessingStatus::FileParseFailed
    )
  }

  pub fn can_transition(&self, next: &FileProcessingStatus) -> bool {
    if self.is_error() {
      true
    } else {
      (*next as i32) >= (*self as i32)
    }
  }
}

pub struct FileProcessingStatusRepository {
  kv: Arc<KeyValueStore>,
}

impl FileProcessingStatusRepository {
  pub fn new(kv: Arc<KeyValueStore>) -> Self {
    Self { kv }
  }

  fn key(file_name: &FileName) -> String {
    format!("file_processing_status:{}", file_name.to_string())
  }

  fn file_name_from_key(key: &str) -> Result<FileName> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() != 2 {
      return Err(anyhow!("Invalid key: {}", key));
    }
    FileName::try_from(parts[1])
  }

  pub async fn get_many(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, FileProcessingStatus>> {
    let statuses = self
      .kv
      .get_many(file_names.iter().map(Self::key).collect())
      .await?;
    let mut result = HashMap::new();
    for (file_name, status) in statuses {
      if let Ok(file_name) = Self::file_name_from_key(&file_name) {
        result.insert(file_name, status);
      } else {
        warn!("Invalid key: {}", file_name);
      }
    }
    Ok(result)
  }

  pub async fn delete_many(&self, file_names: Vec<FileName>) -> Result<()> {
    self
      .kv
      .delete_many(file_names.iter().map(Self::key).collect())
      .await
  }

  pub async fn put_many(
    &self,
    input: HashMap<FileName, FileProcessingStatus>,
  ) -> Result<Vec<FileName>> {
    let file_names = input.keys().cloned().collect::<Vec<_>>();
    let current_statuses = self.get_many(file_names).await?;

    let valid_updates = input
      .into_iter()
      .filter_map(|(file_name, status)| {
        if let Some(current_status) = current_statuses.get(&file_name) {
          if current_status.can_transition(&status) {
            Some((file_name, status))
          } else {
            warn!(
              "Invalid status transition: {} -> {}",
              current_status, status
            );
            None
          }
        } else {
          Some((file_name, status))
        }
      })
      .collect::<HashMap<_, _>>();
    let updated = valid_updates.keys().cloned().collect::<Vec<_>>();

    self
      .kv
      .set_many(
        valid_updates
          .into_iter()
          .map(|(file_name, status)| (Self::key(&file_name), status, None))
          .collect(),
      )
      .await?;

    Ok(updated)
  }
}
