use super::{
  super::file_processing_status::FileProcessingStatusRepository,
  list_lookup::ListLookup,
  list_lookup_repository::{ListLookupRecord, ListLookupRepository, ListSegmentReadModel},
};
use crate::{
  crawler::crawler::{Crawler, QueuePushParametersBuilder},
  events::{
    event::{Event, EventPayloadBuilder, Topic},
    event_publisher::EventPublisher,
  },
  files::file_metadata::{
    file_name::{FileName, ListRootFileName},
    page_type::PageType,
  },
  helpers::priority::Priority,
  lookup::file_processing_status::FileProcessingStatus,
  sqlite::SqliteConnection,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
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

  async fn draft_many_lookups(
    &self,
    lookup_records: Vec<ListLookupRecord>,
  ) -> Result<HashMap<ListRootFileName, ListLookup>> {
    let segment_map = self
      .list_lookup_repository
      .find_many_segments_by_root(
        lookup_records
          .iter()
          .map(|record| record.root_file_name.clone())
          .collect(),
      )
      .await?;

    let mut segment_file_names = HashMap::<ListRootFileName, HashSet<FileName>>::new();
    let mut segment_albums = HashMap::<ListRootFileName, HashMap<FileName, Vec<FileName>>>::new();
    let mut segment_components = HashMap::<ListRootFileName, HashSet<FileName>>::new();

    for (root_file_name, segments) in segment_map {
      let mut root_segment_file_names = HashSet::new();
      let mut root_segment_albums = HashMap::new();
      let mut root_components = HashSet::new();

      for segment in segments {
        let file_name = segment.file_name.clone();
        let albums = segment.albums.clone();
        root_segment_file_names.insert(file_name.clone());
        root_segment_file_names.extend(segment.other_segments.clone());
        root_components.insert(file_name.clone());
        root_components.extend(segment.other_segments);
        root_segment_albums.insert(file_name, albums.clone());
        root_components.extend(albums);
      }

      segment_file_names.insert(root_file_name.clone(), root_segment_file_names);
      segment_albums.insert(root_file_name.clone(), root_segment_albums);
      segment_components.insert(root_file_name.clone(), root_components);
    }

    let mut component_processing_statuses = self
      .file_processing_status_repository
      .get_many(
        segment_components
          .iter()
          .flat_map(|(_, components)| components.clone())
          .collect(),
      )
      .await?;

    let mut lookups = HashMap::new();
    for lookup_record in lookup_records {
      lookups.insert(
        lookup_record.root_file_name.clone(),
        ListLookup {
          root_file_name: lookup_record.root_file_name.clone(),
          segment_albums: segment_albums
            .remove(&lookup_record.root_file_name)
            .unwrap_or_default(),
          component_processing_statuses: segment_components
            .remove(&lookup_record.root_file_name)
            .unwrap_or(HashSet::new())
            .iter()
            .map(|file_name| {
              (
                file_name.clone(),
                component_processing_statuses
                  .remove(file_name)
                  .unwrap_or(FileProcessingStatus::CrawlEnqueued),
              )
            })
            .collect(),
          segment_file_names: segment_file_names
            .remove(&lookup_record.root_file_name)
            .unwrap_or_default()
            .into_iter()
            .collect(),
          last_run: lookup_record.latest_run,
          last_run_status: Some(lookup_record.latest_status),
        },
      );
    }

    Ok(lookups)
  }

  pub async fn run_lookups(
    &self,
    lookups: Vec<ListLookup>,
  ) -> Result<HashMap<ListRootFileName, ListLookup>> {
    let mut outputs = HashMap::new();
    let mut dormant_lookups = HashMap::new();
    let mut dormant_components = HashMap::new();
    let mut dormant_lookup_roots = HashSet::new();

    for lookup in lookups {
      if lookup.is_complete() {
        outputs.insert(lookup.root_file_name.clone(), lookup);
        continue;
      }

      let lookup_dormant_components = lookup.dormant_components();

      if lookup_dormant_components.is_empty() {
        outputs.insert(lookup.root_file_name.clone(), lookup);
        continue;
      }

      dormant_lookup_roots.insert(lookup.root_file_name.clone());
      dormant_components.insert(lookup.root_file_name.clone(), lookup_dormant_components);
      dormant_lookups.insert(lookup.root_file_name.clone(), lookup);
    }

    if !dormant_components.is_empty() {
      self
        .crawler
        .enqueue_many(
          dormant_components
            .iter()
            .flat_map(|(root_file_name, dormant_components)| {
              dormant_components
                .iter()
                .map(|file_name| {
                  let priority = if matches!(file_name.page_type(), PageType::ListSegment) {
                    Priority::Express
                  } else {
                    Priority::High
                  };
                  QueuePushParametersBuilder::default()
                    .file_name(file_name.clone())
                    .priority(priority)
                    .correlation_id(format!("list_lookup:{}", root_file_name.to_string()))
                    .build()
                })
                .collect::<Vec<_>>()
            })
            .collect::<Result<Vec<_>, _>>()?,
        )
        .await?;

      let mut status_updates = dormant_components
        .into_iter()
        .map(|(root_file_name, dormant_components)| {
          (
            root_file_name,
            dormant_components
              .into_iter()
              .map(|file_name| (file_name, FileProcessingStatus::CrawlEnqueued))
              .collect::<Vec<_>>(),
          )
        })
        .collect::<HashMap<_, _>>();

      self
        .file_processing_status_repository
        .put_many(
          status_updates
            .iter()
            .flat_map(|(_, updates)| updates.clone())
            .collect(),
        )
        .await?;

      for (root_file_name, mut lookup) in dormant_lookups.drain() {
        lookup
          .component_processing_statuses
          .extend(status_updates.remove(&root_file_name).unwrap_or_default());
        outputs.insert(root_file_name, lookup);
      }
    }

    let latest_run = Utc::now().naive_utc();
    let mut record_update_payloads = vec![];
    let mut event_payloads = vec![];

    for (root_file_name, lookup) in outputs.iter() {
      let status_updated = lookup.status_updated();
      let processed = dormant_lookup_roots.contains(root_file_name);
      let should_update_record = status_updated || processed;

      if status_updated {
        event_payloads.push(
          EventPayloadBuilder::default()
            .key(root_file_name.to_string())
            .event(Event::ListLookupStatusUpdated {
              root_file_name: root_file_name.clone(),
              status: lookup.status(),
            })
            .build()?,
        );
      }

      if should_update_record {
        record_update_payloads.push((
          root_file_name.clone(),
          lookup.status(),
          if processed { Some(latest_run) } else { None },
        ));
      }
    }

    if !record_update_payloads.is_empty() {
      self
        .list_lookup_repository
        .update_many_lookup_records(record_update_payloads)
        .await?;
    }

    if !event_payloads.is_empty() {
      self
        .event_publisher
        .publish_many(Topic::Lookup, event_payloads)
        .await?;
    }

    Ok(outputs)
  }

  async fn run_lookups_records(
    &self,
    records: Vec<ListLookupRecord>,
  ) -> Result<HashMap<ListRootFileName, ListLookup>> {
    let mut drafts = self.draft_many_lookups(records).await?;
    self
      .run_lookups(drafts.drain().map(|(_, lookup)| lookup).collect())
      .await
  }

  pub async fn run_lookups_containing_components(
    &self,
    components: Vec<FileName>,
  ) -> Result<Vec<ListLookup>> {
    let records = self
      .list_lookup_repository
      .find_lookups_containing_components(components)
      .await?;

    if records.is_empty() {
      return Ok(vec![]);
    }

    Ok(
      self
        .run_lookups_records(records)
        .await?
        .into_iter()
        .map(|(_, lookup)| lookup)
        .collect(),
    )
  }

  pub async fn put_lookup(&self, root_file_name: ListRootFileName) -> Result<ListLookup> {
    let record = self
      .list_lookup_repository
      .put_lookup_record(root_file_name.clone())
      .await?;

    let lookup = self
      .run_lookups_records(vec![record])
      .await?
      .remove(&root_file_name)
      .ok_or(anyhow!("Unexpected error: Failed to run lookup"))?;

    Ok(lookup)
  }

  pub async fn delete_lookup(&self, root_file_name: ListRootFileName) -> Result<()> {
    self
      .list_lookup_repository
      .delete_many_lookups(vec![root_file_name])
      .await
  }
}
