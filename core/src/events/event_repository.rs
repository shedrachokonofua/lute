use super::event::{Event, EventPayload, EventPayloadBuilder, Topic};
use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use rusqlite::{params, types::Value, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc, sync::Arc};
use strum::EnumString;
use tracing::{error, info, instrument};

#[derive(Debug, Clone)]
pub struct EventRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

#[derive(
  Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum_macros::Display, EnumString,
)]
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
  pub topic: Topic,
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
      .key(row.get::<_, Option<String>>(6)?.unwrap_or("".to_string()))
      .build()
      .map_err(|err| {
        error!(message = err.to_string(), "Failed to build event payload");
        rusqlite::Error::ExecuteReturnedResults
      })?,
    topic: Topic::try_from(row.get::<_, String>(5)?.as_str()).map_err(|err| {
      error!(message = err.to_string(), "Failed to parse stream");
      rusqlite::Error::ExecuteReturnedResults
    })?,
  })
}

impl EventRepository {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  pub async fn put_many(&self, events: Vec<(Topic, EventPayload)>) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let transaction = conn.transaction()?;
        for (stream, payload) in events {
          let mut statement = transaction.prepare(
            "
            INSERT INTO events (correlation_id, causation_id, event, metadata, stream, key) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT (stream, key) DO UPDATE SET
              id = excluded.id,
              correlation_id = excluded.correlation_id,
              causation_id = excluded.causation_id,
              event = excluded.event,
              metadata = excluded.metadata,
              stream = excluded.stream,
              key = excluded.key
            ",
          )?;
          statement.execute((
            &payload.correlation_id,
            &payload.causation_id,
            serde_json::to_string(&payload.event)
              .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            serde_json::to_string(&payload.metadata)
              .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            &stream.to_string(),
            &payload.key,
          ))?;
        }
        transaction.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to put many events");
        anyhow!("Failed to put many events")
      })?
  }

  pub async fn put(&self, stream: Topic, payload: EventPayload) -> Result<()> {
    self.put_many(vec![(stream, payload)]).await
  }

  pub async fn find_by_id(&self, id: &str) -> Result<Option<EventRow>> {
    let id = id.to_string();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let row = conn
          .query_row(
            "
            SELECT id, correlation_id, causation_id, event, metadata, stream, key
            FROM events
            WHERE id = ?1
            ",
            [id],
            map_event_row,
          )
          .optional()?;

        Ok(row)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find event by id");
        anyhow!("Failed to find event by id")
      })?
  }

  pub async fn set_key(&self, id: &str, key: String) -> Result<()> {
    let id = id.to_string();
    let row = self.find_by_id(&id).await?.ok_or_else(|| {
      error!(message = "Event not found", "Failed to set key");
      anyhow!("Failed to set key")
    })?;

    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        let conflicting_id = tx
          .query_row(
            "SELECT id FROM events WHERE stream = ?1 AND key = ?2 AND id != ?3",
            [row.topic.to_string(), key.clone(), id.clone()],
            |row| row.get::<_, u32>(0),
          )
          .optional()?;
        if let Some(cid) = conflicting_id {
          info!(conflicting_id = cid, "Deleting conflicting event");
          tx.execute("DELETE FROM events WHERE id = ?", [cid])?;
        }
        tx.execute("UPDATE events SET key = ?1 WHERE id = ?2", [key, id])?;
        tx.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set key");
        anyhow!("Failed to set key")
      })?
  }

  pub async fn count_events(&self) -> Result<usize> {
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

  pub async fn count_events_without_key(&self) -> Result<usize> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        let mut statement = conn.prepare("SELECT COUNT(*) FROM events WHERE key IS NULL")?;
        let count = statement.query_row([], |row| row.get::<_, i64>(0))?;
        Ok(count as usize)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get event count");
        anyhow!("Failed to get event count")
      })?
  }

  pub async fn count_events_each_topic(&self) -> Result<HashMap<Topic, usize>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        let mut statement = conn.prepare(
          "
          SELECT stream, COUNT(*)
          FROM events
          GROUP BY stream
          ",
        )?;
        let rows = statement
          .query_map([], |row| {
            Ok((
              Topic::try_from(row.get::<_, String>(0)?.as_str()).map_err(|e| {
                error!(message = e.to_string(), "Failed to get event count");
                rusqlite::Error::ExecuteReturnedResults
              })?,
              row.get::<_, i64>(1)? as usize,
            ))
          })?
          .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(rows)
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

  pub async fn get_stream_tails(&self) -> Result<Vec<(Topic, String)>> {
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
              Topic::try_from(row.get::<_, String>(0)?.as_str()).map_err(|e| {
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
    streams: &Vec<Topic>,
    subscriber_id: &str,
    count: usize,
  ) -> Result<EventList> {
    let subscriber_id = subscriber_id.to_string();
    let cursor = self.get_cursor(&subscriber_id).await?;
    let is_global = streams.iter().any(|s| s == &Topic::All);
    let stream_tags = streams
      .iter()
      .map(|s| Value::from(s.to_string()))
      .collect::<Vec<_>>();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        if is_global {
          let mut statement = conn.prepare(
            "
            SELECT id, correlation_id, causation_id, event, metadata, stream, key
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
            SELECT id, correlation_id, causation_id, event, metadata, stream, key
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
  pub async fn set_subscriber_status(
    &self,
    subscriber_id: &str,
    status: EventSubscriberStatus,
  ) -> Result<()> {
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
  pub async fn get_subscriber_status(
    &self,
    subscriber_id: &str,
  ) -> Result<Option<EventSubscriberStatus>> {
    let subscriber_id = subscriber_id.to_string();
    let status = self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        let value = conn
          .query_row(
            "SELECT status FROM event_subscribers WHERE id = ?1",
            [subscriber_id],
            |row| row.get::<_, u32>(0),
          )
          .optional()?;
        Ok::<_, rusqlite::Error>(value)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to check status");
        anyhow!("Failed to check status")
      })??;

    let status = status.map(EventSubscriberStatus::try_from).transpose()?;

    Ok(status)
  }
}
