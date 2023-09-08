use super::event::{EventPayload, Stream};
use crate::settings::Settings;
use anyhow::Result;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EventPublisher {
  pub settings: Arc<Settings>,
  pub sqlite_connection: Arc<tokio_rusqlite::Connection>,
}

impl EventPublisher {
  pub fn new(settings: Arc<Settings>, sqlite_connection: Arc<tokio_rusqlite::Connection>) -> Self {
    Self {
      settings,
      sqlite_connection,
    }
  }

  pub async fn publish(&self, stream: Stream, payload: EventPayload) -> Result<()> {
    self.sqlite_connection.call(move |conn| {
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
    }).await?;
    Ok(())
  }
}
