use super::{
  event::{Event, EventPayloadBuilder, Stream},
  event_subscriber_repository::{EventList, EventSubscriberRepository},
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::{collections::HashMap, sync::Arc};
use tracing::instrument;

pub struct SqliteEventSubscriberRepository {
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
}

impl SqliteEventSubscriberRepository {
  pub fn new(sqlite_connection: Arc<tokio_rusqlite::Connection>) -> Self {
    Self { sqlite_connection }
  }
}

struct EventRow {
  id: i32,
  correlation_id: Option<String>,
  causation_id: Option<String>,
  stream: Stream,
  event: Event,
  created_at: NaiveDateTime,
  metadata: Option<HashMap<String, String>>,
}

#[async_trait]
impl EventSubscriberRepository for SqliteEventSubscriberRepository {
  #[instrument(skip(self))]
  async fn get_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<String> {
    let stream = stream.clone();
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .call(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT cursor
          FROM event_subscriber_cursors
          WHERE subscriber_id = ? AND stream = ?
          ",
        )?;
        let mut rows =
          statement.query_map([subscriber_id, stream.tag()], |row| row.get::<_, u32>(0))?;
        rows
          .next()
          .transpose()
          .map(|cursor| cursor.unwrap_or(0).to_string())
      })
      .await
      .map_err(|e| e.into())
  }

  #[instrument(skip(self))]
  async fn set_cursor(&self, stream: &Stream, subscriber_id: &str, cursor: &str) -> Result<()> {
    let stream = stream.clone();
    let cursor = cursor.to_string();
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .call(move |conn| {
        let mut statement = conn.prepare(
          "
          INSERT INTO event_subscriber_cursors (subscriber_id, stream, cursor)
          VALUES (?, ?, ?)
          ON CONFLICT (subscriber_id, stream) DO UPDATE SET cursor = ?
          ",
        )?;
        statement.execute([subscriber_id, stream.tag(), cursor.clone(), cursor])?;

        Ok(())
      })
      .await?;
    Ok(())
  }

  #[instrument(skip(self))]
  async fn delete_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<()> {
    let stream = stream.clone();
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .call(move |conn| {
        let mut statement = conn.prepare(
          "
          DELETE FROM event_subscriber_cursors
          WHERE subscriber_id = ? AND stream = ?
          ",
        )?;
        statement.execute([subscriber_id, stream.tag()])?;
        Ok(())
      })
      .await?;
    Ok(())
  }

  #[instrument(skip(self))]
  async fn get_events_after_cursor(
    &self,
    stream: &Stream,
    subscriber_id: &str,
    count: usize,
    _block: Option<u64>,
  ) -> Result<EventList> {
    let stream = stream.clone();
    let subscriber_id = subscriber_id.to_string();
    let cursor = self.get_cursor(&stream, &subscriber_id).await?;
    self
      .sqlite_connection
      .call(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT id, correlation_id, causation_id, stream, event, created_at, metadata
          FROM events
          WHERE stream = ? AND id > ?
          ORDER BY id ASC
          LIMIT ?
          ",
        )?;
        let mut rows =
          statement.query_map([&stream.tag(), &cursor, &count.to_string()], |row| {
            Ok(EventRow {
              id: row.get(0)?,
              correlation_id: row.get(1)?,
              causation_id: row.get(2)?,
              stream: Stream::try_from(row.get::<_, String>(3)?)
                .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?,
              event: serde_json::from_str(&row.get::<_, String>(4)?)
                .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?,
              created_at: row.get(5)?,
              metadata: row
                .get::<_, Option<String>>(6)?
                .map(|metadata: String| serde_json::from_str(&metadata).unwrap_or(HashMap::new())),
            })
          })?;
        let mut events = vec![];
        while let Some(row) = rows.next().transpose()? {
          events.push((
            row.id.to_string(),
            EventPayloadBuilder::default()
              .correlation_id(row.correlation_id)
              .causation_id(row.causation_id)
              .event(row.event)
              .metadata(row.metadata)
              .build()
              .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?,
          ));
        }
        Ok(EventList { events })
      })
      .await
      .map_err(|e| e.into())
  }
}
