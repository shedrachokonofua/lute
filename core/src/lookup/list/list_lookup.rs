use super::super::file_processing_status::FileProcessingStatus;
use crate::{
  files::file_metadata::file_name::{FileName, ListRootFileName},
  proto,
};
use std::collections::{HashMap, HashSet};

pub enum ListLookupStatus {
  Started,
  InProgress,
  Completed,
  Failed,
  Invalid,
}

impl From<ListLookupStatus> for proto::ListLookupStatus {
  fn from(val: ListLookupStatus) -> Self {
    match val {
      ListLookupStatus::Started => proto::ListLookupStatus::Started,
      ListLookupStatus::InProgress => proto::ListLookupStatus::InProgress,
      ListLookupStatus::Completed => proto::ListLookupStatus::Completed,
      ListLookupStatus::Failed => proto::ListLookupStatus::Failed,
      ListLookupStatus::Invalid => proto::ListLookupStatus::Invalid,
    }
  }
}

pub struct ListLookup {
  pub root_file_name: ListRootFileName,
  pub segment_file_names: Vec<FileName>,
  pub segment_albums: HashMap<FileName, Vec<FileName>>,
  pub component_processing_statuses: HashMap<FileName, FileProcessingStatus>,
}

impl ListLookup {
  pub fn initialize(root_file_name: ListRootFileName) -> Self {
    Self {
      segment_file_names: vec![(&root_file_name).segment_file_name(1)],
      root_file_name,
      segment_albums: HashMap::new(),
      component_processing_statuses: HashMap::new(),
    }
  }

  pub fn status(&self) -> ListLookupStatus {
    if self.segment_file_names.is_empty() {
      return ListLookupStatus::Invalid;
    }

    if self.segment_file_names.len() > 0 && self.segment_albums.len() == 0 {
      return ListLookupStatus::Started;
    }

    if self.segment_file_names.len() > self.segment_albums.len() {
      return ListLookupStatus::InProgress;
    }

    let mut all_errors = true;
    let mut all_terminal = true;

    for status in self.component_processing_statuses.values() {
      if !matches!(status, FileProcessingStatus::FileParseFailed) {
        all_errors = false;
      }
      if !matches!(
        status,
        FileProcessingStatus::FileParseFailed | FileProcessingStatus::ReadModelUpdated
      ) {
        all_terminal = false;
      }
    }

    if all_errors {
      return ListLookupStatus::Failed;
    }

    if all_terminal {
      return ListLookupStatus::Completed;
    }

    ListLookupStatus::InProgress
  }

  pub fn is_complete(&self) -> bool {
    matches!(self.status(), ListLookupStatus::Completed)
  }

  pub fn components(&self) -> Vec<FileName> {
    let mut components = HashSet::new();
    components.extend(self.segment_file_names.clone());
    components.extend(
      self
        .segment_albums
        .values()
        .flat_map(|album_files| album_files.clone()),
    );
    components.into_iter().collect()
  }

  pub fn dormant_components(&self) -> Vec<FileName> {
    self
      .components()
      .into_iter()
      .filter(|file_name| {
        self
          .component_processing_statuses
          .get(file_name)
          .map(|s| {
            matches!(
              s,
              FileProcessingStatus::CrawlFailed | FileProcessingStatus::CrawlEnqueued
            )
          })
          .unwrap_or(true)
      })
      .collect()
  }
}

impl From<ListLookup> for proto::ListLookup {
  fn from(val: ListLookup) -> Self {
    Self {
      root_file_name: val.root_file_name.to_string(),
      status: Into::<proto::ListLookupStatus>::into(val.status()) as i32,
      segment_file_names: val
        .segment_file_names
        .into_iter()
        .map(|f| f.to_string())
        .collect(),
      segments: val
        .segment_albums
        .into_iter()
        .map(|(k, v)| proto::ListLookupSegment {
          file_name: k.to_string(),
          album_file_names: v.into_iter().map(|f| f.to_string()).collect(),
        })
        .collect(),
      component_processing_statuses: val
        .component_processing_statuses
        .into_iter()
        .map(|(k, v)| {
          (
            k.to_string(),
            Into::<proto::FileProcessingStatus>::into(v) as i32,
          )
        })
        .collect(),
    }
  }
}
