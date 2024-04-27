use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveDateTime};
use rusqlite::{params, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct KeyValueStore {
  sqlite_connection: Arc<SqliteConnection>,
}

impl KeyValueStore {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  pub async fn clear(&self) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(|conn| {
        conn
          .execute("DELETE FROM key_value_store", [])
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to clear key value store");
            e
          })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to clear key value store");
        anyhow!("Failed to clear key value store")
      })??;
    Ok(())
  }

  pub async fn size(&self) -> Result<usize> {
    let size: usize = self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        conn
          .query_row("SELECT COUNT(*) FROM key_value_store", [], |row| {
            row.get::<_, usize>(0)
          })
          .map_err(|e| {
            error!(
              message = e.to_string(),
              "Failed to get key value store size"
            );
            rusqlite::Error::ExecuteReturnedResults
          })
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to get key value store size"
        );
        anyhow!("Failed to get key value store size")
      })??;
    Ok(size)
  }

  pub async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
    let key = key.to_string();
    let req_key = key.clone();
    let result: Option<(Vec<u8>, Option<NaiveDateTime>)> = self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        conn
          .query_row(
            "SELECT value, expires_at FROM key_value_store WHERE key = ?1",
            [req_key],
            |row| {
              let value = row.get::<_, Vec<u8>>(0)?;
              let expires_at = row.get::<_, Option<NaiveDateTime>>(1)?;
              Ok((value, expires_at))
            },
          )
          .optional()
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to get key value");
            rusqlite::Error::ExecuteReturnedResults
          })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get key value");
        anyhow!("Failed to get key value")
      })??;

    if let Some((blob, expires_at)) = result {
      if let Some(expires_at) = expires_at {
        if expires_at < chrono::Utc::now().naive_utc() {
          info!("Key value expired: {}", key);
          self.delete(&key).await?;
          return Ok(None);
        }
      }
      let value: T = serde_json::from_slice(&blob)?;
      Ok(Some(value))
    } else {
      Ok(None)
    }
  }

  pub async fn set<T: Serialize + Send + Sync>(
    &self,
    key: &str,
    value: T,
    ttl: Option<Duration>,
  ) -> Result<()> {
    let expires_at = ttl.map(|ttl| chrono::Utc::now().naive_utc() + ttl);
    let key = key.to_string();
    let value = serde_json::to_vec(&value)?;
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          INSERT INTO key_value_store (key, value, expires_at, updated_at)
          VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
          ON CONFLICT(key) DO UPDATE SET 
            value = excluded.value,
            expires_at = excluded.expires_at,
            updated_at = excluded.updated_at
          ",
        )?;
        statement.execute(params![key, value, expires_at])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set key value");
        anyhow!("Failed to set key value")
      })?
  }

  pub async fn delete(&self, key: &str) -> Result<()> {
    let key = key.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare("DELETE FROM key_value_store WHERE key = ?1")?;
        statement.execute(params![key])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete key value");
        anyhow!("Failed to delete key value")
      })?
  }
}
