use crate::{
  context::ApplicationContext,
  job_executor,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobParametersBuilder, JobProcessorBuilder},
    scheduler_repository::Job,
  },
  sqlite::SqliteConnection,
};
use anyhow::{anyhow, Result};
use chrono::{NaiveDateTime, TimeDelta, Utc};
use rusqlite::{params, types::Value};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, rc::Rc, sync::Arc, time::Duration};
use tracing::{error, info, instrument};

#[derive(Debug)]
pub struct KeyValueStore {
  sqlite_connection: Arc<SqliteConnection>,
}

impl KeyValueStore {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(name = "KeyValueStore::clear", skip(self))]
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

  #[instrument(name = "KeyValueStore::increment", skip(self))]
  pub async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
    let key = key.to_string();
    let value = self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        tx.execute(
          "
          INSERT INTO key_value_store (key, value)
          VALUES (?1, ?2)
          ON CONFLICT (key) DO UPDATE SET
            value = value + excluded.value
          ",
          params![key.clone(), delta],
        )?;
        let value = tx.query_row(
          "SELECT value FROM key_value_store WHERE key = ?",
          params![key],
          |row| row.get::<_, i64>(0),
        )?;
        tx.commit()?;
        Ok::<_, rusqlite::Error>(value)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to increment key value");
        anyhow!("Failed to increment key value")
      })??;
    Ok(value)
  }

  #[instrument(name = "KeyValueStore::size", skip(self))]
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

  #[instrument(name = "KeyValueStore::many_exists", skip(self))]
  pub async fn many_exists(&self, keys: Vec<String>) -> Result<HashMap<String, bool>> {
    let key_params = keys
      .iter()
      .map(|f| Value::from(f.clone()))
      .collect::<Vec<Value>>();
    let results = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            value as k,
            EXISTS (
              SELECT 1 
              FROM key_value_store 
              WHERE 
                key_value_store.key = value 
                AND (expires_at > CURRENT_TIMESTAMP OR expires_at IS NULL) 
            ) as e
          FROM rarray(?1);
          ",
        )?;
        let rows = stmt.query_map([Rc::new(key_params)], |row| {
          let key = row.get::<_, String>(0)?;
          let exists = row.get::<_, bool>(1)?;
          Ok((key, exists))
        })?;
        let mut results = HashMap::new();
        for row in rows {
          let (key, exists) = row?;
          results.insert(key, exists);
        }
        Ok::<_, rusqlite::Error>(results)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to check if keys exist");
        anyhow!("Failed to check if key exist")
      })??;

    Ok(results)
  }

  #[instrument(name = "KeyValueStore::exists", skip(self))]
  pub async fn exists(&self, key: String) -> Result<bool> {
    let exists = self
      .sqlite_connection
      .read()
      .await?
      .interact(|conn| {
        conn
          .query_row(
            "SELECT EXISTS(
              SELECT 1 
              FROM key_value_store 
              WHERE 
                key = ?1 
                AND (expires_at > CURRENT_TIMESTAMP OR expires_at IS NULL)
            )",
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

  #[instrument(name = "KeyValueStore::get_many", skip_all, fields(count = key.len()))]
  pub async fn get_many<T: DeserializeOwned + Send + Sync>(
    &self,
    key: Vec<String>,
  ) -> Result<HashMap<String, T>> {
    let key_params = key
      .iter()
      .map(|f| Value::from(f.clone()))
      .collect::<Vec<Value>>();
    let results = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            key as k,
            CAST(value as BLOB) as v,
            expires_at
          FROM key_value_store
          WHERE key IN rarray(?1)
          ",
        )?;
        let rows = stmt.query_map([Rc::new(key_params)], |row| {
          let key = row.get::<_, String>(0)?;
          let value = row.get::<_, Vec<u8>>(1)?;
          let expires_at = row.get::<_, Option<NaiveDateTime>>(2)?;
          Ok((key, value, expires_at))
        })?;
        let mut results = HashMap::new();
        for row in rows {
          let (key, value, expires_at) = row?;
          results.insert(key, (value, expires_at));
        }
        Ok::<_, rusqlite::Error>(results)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get key value");
        anyhow!("Failed to get key value")
      })??;

    let mut valid_keys = vec![];
    let mut expired_keys = vec![];
    for (key, (_, expires_at)) in results.iter() {
      if let Some(expires_at) = expires_at {
        if *expires_at < Utc::now().naive_utc() {
          expired_keys.push(key.clone());
        } else {
          valid_keys.push(key.clone());
        }
      } else {
        valid_keys.push(key.clone());
      }
    }

    if !expired_keys.is_empty() {
      self.delete_many(expired_keys).await?;
    }

    let results = results
      .into_iter()
      .filter(|(key, _)| valid_keys.contains(key))
      .map(|(key, (value, _))| {
        let value: T = serde_json::from_slice(&value)?;
        Ok((key, value))
      })
      .collect::<Result<HashMap<String, T>>>()?;

    Ok(results)
  }

  #[instrument(name = "KeyValueStore::get", skip(self))]
  pub async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
    self
      .get_many::<T>(vec![key.to_string()])
      .await
      .map(|mut results| results.remove(key))
  }

  #[instrument(name = "KeyValueStore::set_many", skip_all, fields(count = key_values.len()))]
  pub async fn set_many<T: Serialize + Send + Sync>(
    &self,
    key_values: Vec<(String, T, Option<Duration>)>,
  ) -> Result<()> {
    let key_values: Vec<(String, Vec<u8>, Option<NaiveDateTime>)> = key_values
      .into_iter()
      .map(|(key, value, ttl)| {
        let expires_at = ttl.map(|ttl| Utc::now().naive_utc() + ttl);
        let value = serde_json::to_vec(&value).unwrap();
        (key, value, expires_at)
      })
      .collect();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        for (key, value, expires_at) in key_values {
          tx.execute(
            "
            INSERT INTO key_value_store (key, value, expires_at, updated_at)
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET 
              value = excluded.value,
              expires_at = excluded.expires_at,
              updated_at = excluded.updated_at
            ",
            params![key, value, expires_at],
          )?;
        }
        tx.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set key value");
        anyhow!("Failed to set key value")
      })?
  }

  #[instrument(name = "KeyValueStore::set", skip_all)]
  pub async fn set<T: Serialize + Send + Sync>(
    &self,
    key: &str,
    value: T,
    ttl: Option<Duration>,
  ) -> Result<()> {
    self.set_many(vec![(key.to_string(), value, ttl)]).await
  }

  #[instrument(name = "KeyValueStore::delete_many", skip(self))]
  pub async fn delete_many(&self, keys: Vec<String>) -> Result<()> {
    let key_params = keys
      .iter()
      .map(|f| Value::from(f.clone()))
      .collect::<Vec<Value>>();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare("DELETE FROM key_value_store WHERE key IN rarray(?1)")?;
        stmt.execute([Rc::new(key_params)])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete key value");
        anyhow!("Failed to delete key value")
      })?
  }

  #[instrument(name = "KeyValueStore::delete", skip(self))]
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

  #[instrument(name = "KeyValueStore::delete_matching", skip(self))]
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

  #[instrument(name = "KeyValueStore::delete_expired", skip(self))]
  pub async fn delete_expired(&self) -> Result<usize> {
    let count = self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        let count = tx.query_row(
          "SELECT COUNT(*) FROM key_value_store WHERE expires_at < CURRENT_TIMESTAMP",
          [],
          |row| row.get::<_, usize>(0),
        )?;
        tx.execute(
          "DELETE FROM key_value_store WHERE expires_at < CURRENT_TIMESTAMP",
          [],
        )?;
        tx.commit()?;
        Ok::<_, rusqlite::Error>(count)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to delete expired key value"
        );
        anyhow!("Failed to delete expired key value")
      })??;
    Ok(count)
  }

  #[instrument(name = "KeyValueStore::count_matching", skip(self))]
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

async fn delete_expired_keys(_: Job, app_context: Arc<ApplicationContext>) -> Result<()> {
  info!("Executing job, deleting expired key value items");
  let count = app_context.kv.delete_expired().await?;
  info!(count = count, "Deleted expired key value items");
  Ok(())
}

pub async fn setup_kv_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::DeleteExpiredKVItems)
        .app_context(Arc::clone(&app_context))
        .executor(job_executor!(delete_expired_keys))
        .build()?,
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
