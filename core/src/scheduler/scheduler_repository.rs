use super::job_name::JobName;
use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use chrono::{NaiveDateTime, TimeDelta};
use rusqlite::params;
use std::{str::FromStr, sync::Arc};
use tracing::error;

#[derive(Clone)]
pub struct SchedulerRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

#[derive(Debug)]
pub struct SchedulerJobRecord {
  pub id: String,
  pub name: JobName,
  pub next_execution: NaiveDateTime,
  pub last_execution: Option<NaiveDateTime>,
  pub interval_seconds: Option<u32>,
  pub payload: Option<Vec<u8>>,
}

impl SchedulerRepository {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  pub async fn put(&self, record: SchedulerJobRecord) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          INSERT INTO scheduler_jobs (id, name, next_execution, last_execution, interval_seconds, payload)
          VALUES (?, ?, ?, ?, ?, ?)
          ON CONFLICT (id) DO UPDATE SET 
            name = excluded.name,
            next_execution = excluded.next_execution, 
            last_execution = excluded.last_execution, 
            interval_seconds = excluded.interval_seconds,
            payload = excluded.payload
          ",
        )?;
        statement.execute(params![
          record.id,
          record.name.to_string(),
          record.next_execution,
          record.last_execution,
          record.interval_seconds,
          record.payload,
        ])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set cursor");
        anyhow!("Failed to set cursor")
      })?
  }

  pub async fn get_pending_jobs(&self) -> Result<Vec<SchedulerJobRecord>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT id, name, next_execution, last_execution, interval_seconds, payload
          FROM scheduler_jobs
          WHERE next_execution <= datetime('now')
          ",
        )?;
        let rows = statement
          .query_map([], |row| {
            Ok(SchedulerJobRecord {
              id: row.get(0)?,
              name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
              next_execution: row.get(2)?,
              last_execution: row.get(3)?,
              interval_seconds: row.get(4)?,
              payload: row.get(5)?,
            })
          })?
          .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get pending jobs");
        anyhow!("Failed to get pending jobs")
      })?
  }

  pub async fn find_job(&self, job_id: &str) -> Result<Option<SchedulerJobRecord>> {
    let job_id = job_id.to_string();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT id, name, next_execution, last_execution, interval_seconds, payload
          FROM scheduler_jobs
          WHERE id = ?
          ",
        )?;
        let mut rows = statement.query_map([job_id], |row| {
          Ok(SchedulerJobRecord {
            id: row.get(0)?,
            name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
            next_execution: row.get(2)?,
            last_execution: row.get(3)?,
            interval_seconds: row.get(4)?,
            payload: row.get(5)?,
          })
        })?;
        rows.next().transpose().map_err(|e| {
          error!(message = e.to_string(), "Failed to get job");
          anyhow!("Failed to get job")
        })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get job");
        anyhow!("Failed to get job")
      })?
  }

  pub async fn delete_job(&self, job_id: &str) -> Result<()> {
    let job_id = job_id.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare("DELETE FROM scheduler_jobs WHERE id = ?")?;
        statement.execute([job_id])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete job");
        anyhow!("Failed to delete job")
      })?
  }

  pub async fn update_execution_times(
    &self,
    job_id: &str,
    next_execution: NaiveDateTime,
    last_execution: NaiveDateTime,
  ) -> Result<()> {
    let job_id = job_id.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          UPDATE scheduler_jobs
          SET next_execution = ?, last_execution = ?
          WHERE id = ?
          ",
        )?;
        statement.execute(params![next_execution, last_execution, job_id])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to update execution times");
        anyhow!("Failed to update execution times")
      })?
  }

  pub async fn update_job_after_execution(&self, job_id: &str) -> Result<()> {
    let last_execution = chrono::Utc::now().naive_utc();
    let job = self
      .find_job(&job_id)
      .await?
      .ok_or(anyhow!("Job not found"))?;

    if let Some(interval_seconds) = job.interval_seconds {
      let next_execution =
        last_execution + TimeDelta::try_seconds(interval_seconds as i64).expect("Invalid interval");
      self
        .update_execution_times(job_id, next_execution, last_execution)
        .await?;
    } else {
      self.delete_job(&job_id).await?;
    }

    Ok(())
  }
}
