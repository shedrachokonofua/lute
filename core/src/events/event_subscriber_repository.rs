use super::event::{Event, EventPayload, EventPayloadBuilder, Stream};
use anyhow::Result;
use chrono::NaiveDateTime;
use std::{collections::HashMap, sync::Arc};
use tracing::instrument;

pub struct EventSubscriberRepository {
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
}

pub struct EventList {
  pub events: Vec<(String, EventPayload)>,
}

impl EventList {
  pub fn tail_cursor(&self) -> Option<String> {
    self.events.last().map(|(id, _)| id.to_string())
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

impl EventSubscriberRepository {
  pub fn new(sqlite_connection: Arc<tokio_rusqlite::Connection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(skip(self))]
  pub async fn get_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<String> {
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
  pub async fn set_cursor(&self, stream: &Stream, subscriber_id: &str, cursor: &str) -> Result<()> {
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
  pub async fn delete_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<()> {
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
  pub async fn get_events_after_cursor(
    &self,
    stream: &Stream,
    subscriber_id: &str,
    count: usize,
  ) -> Result<EventList> {
    let stream = stream.clone();
    let subscriber_id = subscriber_id.to_string();
    let cursor = self.get_cursor(&stream, &subscriber_id).await?;
    self
      .sqlite_connection
      .call(move |conn| {
        let sql = match stream {
          Stream::Global => {
            "
            SELECT id, correlation_id, causation_id, stream, event, created_at, metadata
            FROM events
            WHERE id > ?
            ORDER BY id ASC
            LIMIT ?
            "
          }
          _ => {
            "
            SELECT id, correlation_id, causation_id, stream, event, created_at, metadata
            FROM events
            WHERE stream = ? AND id > ?
            ORDER BY id ASC
            LIMIT ?
            "
          }
        };
        let params = match stream {
          Stream::Global => (cursor.clone(), count.to_string(), "".to_string()),
          _ => (stream.tag(), cursor.clone(), count.to_string()),
        };

        let mut statement = conn.prepare(sql)?;
        let mut rows = statement.query_map(params, |row| {
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
