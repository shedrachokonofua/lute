use super::{
  album_search::album_search_lookup_event_subscribers::build_album_search_lookup_event_subscribers,
  file_processing_status::FileProcessingStatus,
  list::list_lookup_event_subscribers::build_list_lookup_event_subscribers,
};
use crate::{
  context::ApplicationContext,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
      GroupingStrategy,
    },
  },
  files::file_metadata::file_name::FileName,
  group_event_handler,
};
use anyhow::Result;
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};

fn handle_update(
  updates: &mut HashMap<FileName, FileProcessingStatus>,
  file_name: &FileName,
  next_status: FileProcessingStatus,
) -> () {
  if !updates.contains_key(file_name) {
    updates.insert(file_name.clone(), next_status);
  } else {
    if let Some(status) = updates.get_mut(file_name) {
      if status.can_transition(&next_status) {
        *status = next_status;
      }
    }
  }
}

async fn update_file_processing_status(
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let mut updates = HashMap::new();
  let mut deletions = HashSet::new();

  for data in event_data {
    match data.payload.event {
      Event::CrawlEnqueued { file_name } => {
        deletions.remove(&file_name);
        handle_update(
          &mut updates,
          &file_name,
          FileProcessingStatus::CrawlEnqueued,
        );
      }
      Event::CrawlFailed { file_name, .. } => {
        deletions.remove(&file_name);
        handle_update(&mut updates, &file_name, FileProcessingStatus::CrawlFailed);
      }
      Event::FileSaved { file_name, .. } => {
        deletions.remove(&file_name);
        handle_update(&mut updates, &file_name, FileProcessingStatus::FileSaved);
      }
      Event::FileParsed { file_name, .. } => {
        deletions.remove(&file_name);
        handle_update(&mut updates, &file_name, FileProcessingStatus::FileParsed);
      }
      Event::FileParseFailed { file_name, .. } => {
        deletions.remove(&file_name);
        handle_update(
          &mut updates,
          &file_name,
          FileProcessingStatus::FileParseFailed,
        );
      }
      Event::AlbumSaved { file_name } => {
        deletions.remove(&file_name);
        handle_update(
          &mut updates,
          &file_name,
          FileProcessingStatus::ReadModelUpdated,
        );
      }
      Event::ListSegmentSaved { file_name } => {
        deletions.remove(&file_name);
        handle_update(
          &mut updates,
          &file_name,
          FileProcessingStatus::ReadModelUpdated,
        );
      }
      Event::FileDeleted { file_name, .. } => {
        updates.remove(&file_name);
        deletions.insert(file_name);
      }
      _ => {}
    }
  }

  if !deletions.is_empty() {
    app_context
      .lookup_interactor
      .delete_many_file_processing_status(deletions.iter().cloned().collect())
      .await?;
  }

  if !updates.is_empty() {
    app_context
      .lookup_interactor
      .put_many_file_processing_status(updates)
      .await?;
  }

  Ok(())
}

pub fn build_lookup_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  let mut subscribers = vec![EventSubscriberBuilder::default()
    .id("update_file_processing_status")
    .topic(Topic::All)
    .batch_size(500)
    .app_context(Arc::clone(&app_context))
    .grouping_strategy(GroupingStrategy::All)
    .handler(group_event_handler!(update_file_processing_status))
    .build()?];
  subscribers.extend(build_album_search_lookup_event_subscribers(Arc::clone(
    &app_context,
  ))?);
  subscribers.extend(build_list_lookup_event_subscribers(app_context)?);
  Ok(subscribers)
}
