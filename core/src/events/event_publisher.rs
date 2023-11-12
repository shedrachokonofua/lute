use super::event::{EventPayload, Stream};
use crate::{settings::Settings, sqlite::SqliteConnection};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::error;

#[derive(Debug, Clone)]
pub struct EventPublisher {
  pub settings: Arc<Settings>,
  pub sqlite_connection: Arc<SqliteConnection>,
}

impl EventPublisher {
  pub fn new(settings: Arc<Settings>, sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self {
      settings,
      sqlite_connection,
    }
  }

  pub async fn publish(&self, stream: Stream, payload: EventPayload) -> Result<()> {
    self.sqlite_connection.get().await?.interact(move |conn| {
      conn.execute(
        "INSERT INTO events (correlation_id, causation_id, event, metadata, stream) VALUES (?1, ?2, ?3, ?4, ?5)",
        (
          &payload.correlation_id,
          &payload.causation_id,
          serde_json::to_string(&payload.event)
              .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
          serde_json::to_string(&payload.metadata)
              .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
          &stream.tag(),
        ),
      )?;
      Ok(())
    })
    .await
    .map_err(|e| {
      error!("Failed to publish event: {:?}", e);
      anyhow!("Failed to publish event: {:?}", e)
    })?
  }

  pub async fn batch_publish(&self, stream: Stream, payloads: Vec<EventPayload>) -> Result<()> {
    self.sqlite_connection.get().await?.interact(move |conn| {
      let transaction = conn.transaction()?;
      for payload in payloads {
        let mut statement = transaction.prepare(
        "INSERT INTO events (correlation_id, causation_id, event, metadata, stream) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        statement.execute((
          &payload.correlation_id,
          &payload.causation_id,
          serde_json::to_string(&payload.event)
              .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
          serde_json::to_string(&payload.metadata)
              .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
          &stream.tag(),
        ))?;
      }
      transaction.commit()?;
      Ok(())
    })
    .await
    .map_err(|e| {
      error!("Failed to publish event: {:?}", e);
      anyhow!("Failed to publish event: {:?}", e)
    })?
  }
}
