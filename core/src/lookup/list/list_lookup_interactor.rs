use super::{
  super::file_processing_status::FileProcessingStatusRepository,
  list_lookup::ListLookup,
  list_lookup_repository::{ListLookupRepository, ListSegmentReadModel},
};
use crate::{
  crawler::crawler::{Crawler, QueuePushParametersBuilder},
  events::{
    event::{Event, EventPayloadBuilder, Topic},
    event_publisher::EventPublisher,
  },
  files::file_metadata::{file_name::ListRootFileName, page_type::PageType},
  helpers::priority::Priority,
  lookup::file_processing_status::FileProcessingStatus,
  sqlite::SqliteConnection,
};
use anyhow::Result;
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};

pub struct ListLookupInteractor {
  list_lookup_repository: ListLookupRepository,
  file_processing_status_repository: Arc<FileProcessingStatusRepository>,
  crawler: Arc<Crawler>,
  event_publisher: Arc<EventPublisher>,
}

impl ListLookupInteractor {
  pub fn new(
    file_processing_status_repository: Arc<FileProcessingStatusRepository>,
    sqlite_connection: Arc<SqliteConnection>,
    crawler: Arc<Crawler>,
    event_publisher: Arc<EventPublisher>,
  ) -> Self {
    Self {
      list_lookup_repository: ListLookupRepository::new(sqlite_connection),
      file_processing_status_repository,
      crawler,
      event_publisher,
    }
  }

  pub async fn put_many_list_segments(&self, docs: Vec<ListSegmentReadModel>) -> Result<()> {
    let file_names = docs
      .iter()
      .map(|doc| doc.file_name.clone())
      .collect::<Vec<_>>();
    self.list_lookup_repository.put_many_segments(docs).await?;
    self
      .event_publisher
      .publish_many(
        Topic::Lookup,
        file_names
          .into_iter()
          .map(|file_name| {
            EventPayloadBuilder::default()
              .key(format!("segment_saved:{}", file_name.to_string()))
              .event(Event::ListSegmentSaved { file_name })
              .build()
          })
          .collect::<Result<Vec<_>, _>>()?,
      )
      .await?;
    Ok(())
  }

  pub async fn draft_list_lookup(&self, root_file_name: ListRootFileName) -> Result<ListLookup> {
    let segment_docs = self
      .list_lookup_repository
      .find_many_segments_by_root(root_file_name.clone())
      .await?;

    if segment_docs.is_empty() {
      return Ok(ListLookup::initialize(root_file_name));
    }

    let mut segment_file_names = HashSet::new();
    let mut segment_albums = HashMap::new();
    let mut components = HashSet::new();

    for doc in segment_docs {
      let file_name = doc.file_name.clone();
      let albums = doc.albums.clone();
      segment_file_names.insert(file_name.clone());
      segment_file_names.extend(doc.other_segments.clone());
      components.insert(file_name.clone());
      components.extend(doc.other_segments);
      segment_albums.insert(file_name, albums.clone());
      components.extend(albums);
    }

    let component_processing_statuses = self
      .file_processing_status_repository
      .get_many(components.into_iter().collect())
      .await?;

    Ok(ListLookup {
      root_file_name,
      segment_albums,
      component_processing_statuses,
      segment_file_names: segment_file_names.into_iter().collect(),
    })
  }

  // pub async fn poll_lookup(&self, root_file_name: ListRootFileName) -> Result<ListLookup> {}

  pub async fn put_lookup(&self, root_file_name: ListRootFileName) -> Result<ListLookup> {
    self
      .list_lookup_repository
      .put_lookup(root_file_name.clone())
      .await?;
    let mut lookup = self.draft_list_lookup(root_file_name).await?;

    if lookup.is_complete() {
      return Ok(lookup);
    }

    let dormant_components = lookup.dormant_components();

    if dormant_components.is_empty() {
      return Ok(lookup);
    }

    for file_name in dormant_components.iter() {
      let priority = if matches!(file_name.page_type(), PageType::ListSegment) {
        Priority::Express
      } else {
        Priority::High
      };
      self
        .crawler
        .enqueue(
          QueuePushParametersBuilder::default()
            .file_name(file_name.clone())
            .priority(priority)
            .build()?,
        )
        .await?;
    }

    let updates = dormant_components
      .into_iter()
      .map(|file_name| (file_name.clone(), FileProcessingStatus::CrawlEnqueued))
      .collect::<HashMap<_, _>>();

    self
      .file_processing_status_repository
      .put_many(updates.clone())
      .await?;

    lookup.component_processing_statuses.extend(updates);

    Ok(lookup)
  }
}
