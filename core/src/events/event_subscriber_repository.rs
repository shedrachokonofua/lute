use super::event::{Event, EventPayload, EventPayloadBuilder, Stream, StreamKind};
use anyhow::Result;
use rusqlite::{params, types::Value};
use std::{collections::HashMap, rc::Rc, sync::Arc};
use tracing::{error, instrument};

pub struct EventSubscriberRepository {
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
}

#[derive(Debug, Clone)]
pub struct EventRow {
  pub id: String,
  pub stream: Stream,
  pub payload: EventPayload,
}

pub struct EventList {
  pub rows: Vec<EventRow>,
}

impl EventList {
  pub fn tail_cursor(&self) -> Option<String> {
    self.rows.last().map(|row| row.id.clone())
  }
}

fn map_event_row(row: &rusqlite::Row<'_>) -> Result<EventRow, rusqlite::Error> {
  Ok(EventRow {
    id: row.get::<_, i32>(0)?.to_string(),
    payload: EventPayloadBuilder::default()
      .correlation_id(row.get::<_, Option<String>>(1)?)
      .causation_id(row.get::<_, Option<String>>(2)?)
      .event(
        serde_json::from_str::<Event>(&row.get::<_, String>(3)?).map_err(|err| {
          error!(message = err.to_string(), "Failed to deserialize event");
          rusqlite::Error::ExecuteReturnedResults
        })?,
      )
      .metadata(
        row
          .get::<_, Option<String>>(4)?
          .map(|metadata: String| serde_json::from_str(&metadata).unwrap_or(HashMap::new())),
      )
      .build()
      .map_err(|err| {
        error!(message = err.to_string(), "Failed to build event payload");
        rusqlite::Error::ExecuteReturnedResults
      })?,
    stream: Stream::try_from(row.get::<_, String>(5)?).map_err(|err| {
      error!(message = err.to_string(), "Failed to parse stream");
      rusqlite::Error::ExecuteReturnedResults
    })?,
  })
}

impl EventSubscriberRepository {
  pub fn new(sqlite_connection: Arc<tokio_rusqlite::Connection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(skip(self))]
  pub async fn get_cursor(&self, subscriber_id: &str) -> Result<String> {
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .call(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT cursor
          FROM event_subscriber_cursors
          WHERE subscriber_id = ?
          ",
        )?;
        let mut rows = statement.query_map([subscriber_id], |row| row.get::<_, u32>(0))?;
        rows
          .next()
          .transpose()
          .map(|cursor| cursor.unwrap_or(0).to_string())
      })
      .await
      .map_err(|e| e.into())
  }

  #[instrument(skip(self))]
  pub async fn set_cursor(&self, subscriber_id: &str, cursor: &str) -> Result<()> {
    let cursor = cursor.to_string();
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .call(move |conn| {
        let mut statement = conn.prepare(
          "
          INSERT INTO event_subscriber_cursors (subscriber_id, cursor)
          VALUES (?1, ?2)
          ON CONFLICT (subscriber_id) DO UPDATE SET cursor = ?2
          ",
        )?;
        statement.execute(params![subscriber_id, cursor])?;

        Ok(())
      })
      .await?;
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn delete_cursor(&self, subscriber_id: &str) -> Result<()> {
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .call(move |conn| {
        let mut statement = conn.prepare(
          "
          DELETE FROM event_subscriber_cursors
          WHERE subscriber_id = ?
          ",
        )?;
        statement.execute([subscriber_id])?;
        Ok(())
      })
      .await?;
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn get_events_after_cursor(
    &self,
    streams: &Vec<Stream>,
    subscriber_id: &str,
    count: usize,
  ) -> Result<EventList> {
    let subscriber_id = subscriber_id.to_string();
    let cursor = self.get_cursor(&subscriber_id).await?;
    let is_global = streams.iter().any(|s| s.kind() == StreamKind::Global);
    let stream_tags = streams
      .iter()
      .map(|s| Value::from(s.tag()))
      .collect::<Vec<_>>();
    self
      .sqlite_connection
      .call(move |conn| {
        if is_global {
          let mut statement = conn.prepare(
            "
            SELECT id, correlation_id, causation_id, event, metadata, stream
            FROM events
            WHERE id > ?1
            ORDER BY id ASC
            LIMIT ?2
            ",
          )?;
          let rows = statement
            .query_map(params![cursor.clone(), count.to_string()], map_event_row)?
            .collect::<Result<Vec<_>, _>>()?;
          Ok(EventList { rows })
        } else {
          let mut statement = conn.prepare(
            "
            SELECT id, correlation_id, causation_id, event, metadata, stream
            FROM events
            WHERE stream IN rarray(?1) AND id > ?2
            ORDER BY id ASC
            LIMIT ?3
            ",
          )?;
          let rows = statement
            .query_map(
              params![Rc::new(stream_tags), cursor.clone(), count.to_string()],
              map_event_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;
          Ok(EventList { rows })
        }
      })
      .await
      .map_err(|e| e.into())
  }
}
