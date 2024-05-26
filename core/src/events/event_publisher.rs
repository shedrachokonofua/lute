use super::{
  event::{EventPayload, Topic},
  event_repository::EventRepository,
};
use crate::{settings::Settings, sqlite::SqliteConnection};
use anyhow::Result;
use std::sync::Arc;

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
