use super::{
  event::{Event, EventPayload, Topic},
  event_repository::EventRepository,
  event_subscriber::{
    EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
  },
};
use crate::{
  context::ApplicationContext, event_handler,
  lookup::album_search_lookup::get_album_search_correlation_id, settings::Settings,
  sqlite::SqliteConnection,
};
use anyhow::Result;
use chrono::TimeDelta;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone)]
pub struct EventPublisher {
  pub settings: Arc<Settings>,
  pub event_repository: EventRepository,
}

impl EventPublisher {
  pub fn new(settings: Arc<Settings>, sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self {
      settings,
      event_repository: EventRepository::new(sqlite_connection),
    }
  }

  pub async fn publish(&self, stream: Topic, payload: EventPayload) -> Result<()> {
    self.batch_publish(stream, vec![payload]).await
  }

  pub async fn batch_publish(&self, stream: Topic, payloads: Vec<EventPayload>) -> Result<()> {
    self
      .event_repository
      .put_many(
        payloads
          .into_iter()
          .map(|payload| (stream.clone(), payload))
          .collect(),
      )
      .await
  }
}

pub async fn set_event_key(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let key = match event_data.payload.event {
    Event::ProfileAlbumAdded {
      profile_id,
      file_name,
      ..
    } => {
      format!("{}:{}", profile_id.to_string(), file_name.to_string())
    }
    Event::FileSaved { file_name, .. } => file_name.to_string(),
    Event::FileDeleted { file_name, .. } => file_name.to_string(),
    Event::FileParsed { file_name, .. } => file_name.to_string(),
    Event::FileParseFailed { file_name, .. } => file_name.to_string(),
    Event::LookupAlbumSearchUpdated { lookup } => get_album_search_correlation_id(lookup.query()),
  };
  info!(
    event_id = event_data.entry_id.to_string(),
    key = &key,
    "Setting event key"
  );
  let event_repository = EventRepository::new(Arc::clone(&app_context.sqlite_connection));
  event_repository.set_key(&event_data.entry_id, key).await?;
  Ok(())
}

pub fn build_event_key_migration_subscriber(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![EventSubscriberBuilder::default()
    .id("event_key_migration")
    .app_context(app_context)
    .topic(Topic::All)
    .handler(event_handler!(set_event_key))
    .cooldown(TimeDelta::try_milliseconds(50).unwrap().to_std()?)
    .build()?])
}
