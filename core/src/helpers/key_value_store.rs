use crate::{
  context::ApplicationContext,
  scheduler::{job_name::JobName, scheduler::JobParametersBuilder},
  sqlite::SqliteConnection,
};
use anyhow::{anyhow, Result};
use chrono::{NaiveDateTime, TimeDelta, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct KeyValueStore {
  sqlite_connection: Arc<SqliteConnection>,
}

impl KeyValueStore {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  #[tracing::instrument(name = "KeyValueStore::clear", skip(self))]
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

  #[tracing::instrument(name = "KeyValueStore::size", skip(self))]
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

  #[tracing::instrument(name = "KeyValueStore::exists", skip(self))]
  pub async fn exists(&self, key: String) -> Result<bool> {
    let exists = self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        conn
          .query_row(
            "SELECT EXISTS(SELECT 1 FROM key_value_store WHERE key = ?1)",
            [key],
            |row| row.get::<_, bool>(0),
          )
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to check if key exists");
            rusqlite::Error::ExecuteReturnedResults
          })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to check if key exists");
        anyhow!("Failed to check if key exists")
      })??;
    Ok(exists)
  }

  #[tracing::instrument(name = "KeyValueStore::get", skip(self))]
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
        if expires_at < Utc::now().naive_utc() {
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

  #[tracing::instrument(name = "KeyValueStore::set", skip_all)]
  pub async fn set<T: Serialize + Send + Sync>(
    &self,
    key: &str,
    value: T,
    ttl: Option<Duration>,
  ) -> Result<()> {
    let expires_at = ttl.map(|ttl| Utc::now().naive_utc() + ttl);
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

  #[tracing::instrument(name = "KeyValueStore::delete", skip(self))]
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

  #[tracing::instrument(name = "KeyValueStore::delete_matching", skip(self))]
  pub async fn delete_matching(&self, pattern: &str) -> Result<()> {
    let pattern = pattern.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare("DELETE FROM key_value_store WHERE key LIKE ?1")?;
        statement.execute(params![pattern])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete key value");
        anyhow!("Failed to delete key value")
      })?
  }

  #[tracing::instrument(name = "KeyValueStore::delete_expired", skip(self))]
  pub async fn delete_expired(&self) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement =
          conn.prepare("DELETE FROM key_value_store WHERE expires_at < CURRENT_TIMESTAMP")?;
        statement.execute([])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to delete expired key value"
        );
        anyhow!("Failed to delete expired key value")
      })?
  }

  #[tracing::instrument(name = "KeyValueStore::count_matching", skip(self))]
  pub async fn count_matching(&self, pattern: &str) -> Result<usize> {
    let pattern = pattern.to_string();
    let count: usize = self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        conn
          .query_row(
            "SELECT COUNT(*) FROM key_value_store WHERE key LIKE ?1",
            [pattern],
            |row| row.get::<_, usize>(0),
          )
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to count key value");
            rusqlite::Error::ExecuteReturnedResults
          })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to count key value");
        anyhow!("Failed to count key value")
      })??;
    Ok(count)
  }
}

pub async fn setup_kv_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobName::DeleteExpiredKVItems,
      Arc::new(|ctx| {
        Box::pin(async move {
          info!("Executing job, deleting expired key value items");
          ctx.app_context.kv.delete_expired().await
        })
      }),
    )
    .await;

  app_context
    .scheduler
    .put(
      JobParametersBuilder::default()
        .name(JobName::DeleteExpiredKVItems)
        .interval(TimeDelta::try_hours(1).unwrap())
        .build()?,
    )
    .await?;

  Ok(())
}
