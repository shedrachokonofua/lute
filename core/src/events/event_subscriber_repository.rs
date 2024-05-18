use super::event::{Event, EventPayload, EventPayloadBuilder, Stream, StreamKind};
use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use rusqlite::{params, types::Value};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc, sync::Arc};
use tracing::{error, instrument};

#[derive(Debug, Clone)]
pub struct EventSubscriberRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSubscriberStatus {
  Paused = 0,
  Running = 1,
}

impl TryFrom<u32> for EventSubscriberStatus {
  type Error = anyhow::Error;

  fn try_from(value: u32) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(EventSubscriberStatus::Paused),
      1 => Ok(EventSubscriberStatus::Running),
      _ => Err(anyhow!("Invalid event subscriber status")),
    }
  }
}

#[derive(Debug, Clone)]
pub struct EventSubscriberRow {
  pub id: String,
  pub cursor: String,
  pub status: EventSubscriberStatus,
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
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  pub async fn get_event_count(&self) -> Result<usize> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        let mut statement = conn.prepare("SELECT COUNT(*) FROM events")?;
        let count = statement.query_row([], |row| row.get::<_, i64>(0))?;
        Ok(count as usize)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get event count");
        anyhow!("Failed to get event count")
      })?
  }

  pub async fn get_subscribers(&self) -> Result<Vec<EventSubscriberRow>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        let mut statement = conn.prepare("SELECT id, cursor, status FROM event_subscribers")?;
        let rows = statement
          .query_map([], |row| {
            Ok(EventSubscriberRow {
              id: row.get::<_, String>(0)?,
              cursor: row.get::<_, u32>(1)?.to_string(),
              status: EventSubscriberStatus::try_from(row.get::<_, u32>(2)?).map_err(|e| {
                error!(message = e.to_string(), "Failed to get subscribers");
                rusqlite::Error::ExecuteReturnedResults
              })?,
            })
          })?
          .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get subscribers");
        anyhow!("Failed to get subscribers")
      })?
  }

  pub async fn get_stream_tails(&self) -> Result<Vec<(Stream, String)>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        let mut statement = conn.prepare(
          "
          SELECT stream, MAX(id)
          FROM events
          GROUP BY stream
          ",
        )?;
        let rows = statement
          .query_map([], |row| {
            Ok((
              Stream::try_from(row.get::<_, String>(0)?).map_err(|e| {
                error!(message = e.to_string(), "Failed to get stream tails");
                rusqlite::Error::ExecuteReturnedResults
              })?,
              row.get::<_, i32>(1)?.to_string(),
            ))
          })?
          .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get stream tails");
        anyhow!("Failed to get stream tails")
      })?
  }

  #[instrument(skip(self))]
  pub async fn get_cursor(&self, subscriber_id: &str) -> Result<String> {
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare("SELECT cursor FROM event_subscribers WHERE id = ?")?;
        let mut rows = statement.query_map([subscriber_id], |row| row.get::<_, u32>(0))?;
        rows
          .next()
          .transpose()
          .map(|cursor| cursor.unwrap_or(0).to_string())
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to get cursor");
            anyhow::anyhow!("Failed to get cursor")
          })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get cursor");
        anyhow::anyhow!("Failed to get cursor")
      })?
  }

  #[instrument(skip(self))]
  pub async fn set_cursor(&self, subscriber_id: &str, cursor: &str) -> Result<()> {
    let cursor = cursor.to_string();
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          INSERT INTO event_subscribers (id, cursor)
          VALUES (?1, ?2)
          ON CONFLICT (id) DO UPDATE SET cursor = ?2
          ",
        )?;
        statement.execute(params![subscriber_id, cursor])?;

        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set cursor");
        anyhow::anyhow!("Failed to set cursor")
      })?
  }

  #[instrument(skip(self))]
  pub async fn delete_cursor(&self, subscriber_id: &str) -> Result<()> {
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare("DELETE FROM event_subscribers WHERE id = ?")?;
        statement.execute([subscriber_id])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete cursor");
        anyhow::anyhow!("Failed to delete cursor")
      })?
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
      .read()
      .await?
      .interact(move |conn| {
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
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get events after cursor");
        anyhow!("Failed to get events after cursor")
      })?
  }

  #[instrument(skip(self))]
  pub async fn set_status(&self, subscriber_id: &str, status: EventSubscriberStatus) -> Result<()> {
    let subscriber_id = subscriber_id.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          UPDATE event_subscribers
          SET status = ?2
          WHERE id = ?1
          ",
        )?;
        statement.execute(params![subscriber_id, status as u32])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set subscriber status");
        anyhow!("Failed to set subscriber status")
      })?
  }

  #[instrument(skip(self))]
  pub async fn get_status(&self, subscriber_id: &str) -> Result<Option<EventSubscriberStatus>> {
    let subscriber_id = subscriber_id.to_string();
    let status = self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        conn
          .query_row(
            "SELECT status FROM event_subscribers WHERE id = ?1",
            [subscriber_id],
            |row| row.get::<_, Option<u32>>(0),
          )
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to check if key exists");
            rusqlite::Error::ExecuteReturnedResults
          })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to check status");
        anyhow!("Failed to check status")
      })??;

    let status = status
      .map(|s| EventSubscriberStatus::try_from(s))
      .transpose()?;

    Ok(status)
  }
}
