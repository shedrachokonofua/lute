use super::job_name::JobName;
use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveDateTime, TimeDelta};
use rusqlite::{params, types::Value};
use std::{rc::Rc, str::FromStr, sync::Arc};
use tracing::error;

#[derive(Clone)]
pub struct SchedulerRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

#[derive(Debug, Clone)]
pub struct Job {
  pub id: String,
  pub name: JobName,
  pub next_execution: NaiveDateTime,
  pub last_execution: Option<NaiveDateTime>,
  pub interval_seconds: Option<u32>,
  pub payload: Option<Vec<u8>>,
  pub claimed_at: Option<NaiveDateTime>,
}

impl SchedulerRepository {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  pub async fn put(&self, record: Job) -> Result<()> {
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

  pub async fn get_jobs(&self) -> Result<Vec<Job>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT id, name, next_execution, last_execution, interval_seconds, payload, claimed_at
          FROM scheduler_jobs
          ",
        )?;
        let rows = statement
          .query_map([], |row| {
            Ok(Job {
              id: row.get(0)?,
              name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
              next_execution: row.get(2)?,
              last_execution: row.get(3)?,
              interval_seconds: row.get(4)?,
              payload: row.get(5)?,
              claimed_at: row.get(6)?,
            })
          })?
          .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get all jobs");
        anyhow!("Failed to get all jobs")
      })?
  }

  pub async fn set_many_claimed_at(
    &self,
    job_ids: Vec<String>,
    claimed_at: NaiveDateTime,
  ) -> Result<()> {
    let ids = job_ids.into_iter().map(Value::from).collect::<Vec<_>>();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          UPDATE scheduler_jobs
          SET claimed_at = ?
          WHERE id IN rarray(?)
          ",
        )?;
        statement.execute(params![claimed_at, Rc::new(ids)])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set claimed at");
        anyhow!("Failed to set claimed at")
      })?
  }

  pub async fn set_claimed_at(&self, job_id: String, claimed_at: NaiveDateTime) -> Result<()> {
    self.set_many_claimed_at(vec![job_id], claimed_at).await
  }

  pub async fn claim_next_jobs(
    &self,
    job_name: JobName,
    count: u32,
    claim_duration: Duration,
  ) -> Result<Vec<Job>> {
    let oldest_claimed_at = chrono::Utc::now().naive_utc() - claim_duration;
    let jobs = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT id, name, next_execution, last_execution, interval_seconds, payload, claimed_at
          FROM scheduler_jobs
          WHERE
            name = ?
            AND next_execution <= datetime('now')
            AND (
              claimed_at IS NULL
              OR claimed_at < datetime(?)
            )
          ORDER BY next_execution, id
          LIMIT ?
          ",
        )?;
        let rows = statement
          .query_map(
            params![job_name.to_string(), oldest_claimed_at, count],
            |row| {
              Ok(Job {
                id: row.get(0)?,
                name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
                next_execution: row.get(2)?,
                last_execution: row.get(3)?,
                interval_seconds: row.get(4)?,
                payload: row.get(5)?,
                claimed_at: row.get(6)?,
              })
            },
          )?
          .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to claim next job");
        anyhow!("Failed to claim next job")
      })??;

    if !jobs.is_empty() {
      self
        .set_many_claimed_at(
          jobs.iter().map(|job| job.id.clone()).collect(),
          chrono::Utc::now().naive_utc(),
        )
        .await?;
    }

    Ok(jobs)
  }

  pub async fn find_jobs(&self, job_ids: Vec<String>) -> Result<Vec<Job>> {
    let ids = job_ids.into_iter().map(Value::from).collect::<Vec<_>>();
    let jobs = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT id, name, next_execution, last_execution, interval_seconds, payload, claimed_at
          FROM scheduler_jobs
          WHERE id IN rarray(?)
          ",
        )?;
        let rows = statement
          .query_map(params![Rc::new(ids)], |row| {
            Ok(Job {
              id: row.get(0)?,
              name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
              next_execution: row.get(2)?,
              last_execution: row.get(3)?,
              interval_seconds: row.get(4)?,
              payload: row.get(5)?,
              claimed_at: row.get(6)?,
            })
          })?
          .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to claim next job");
        anyhow!("Failed to claim next job")
      })??;

    Ok(jobs)
  }

  pub async fn find_job(&self, job_id: &str) -> Result<Option<Job>> {
    let job_id = job_id.to_string();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT id, name, next_execution, last_execution, interval_seconds, payload, claimed_at
          FROM scheduler_jobs
          WHERE id = ?
          ",
        )?;
        let mut rows = statement.query_map([job_id], |row| {
          Ok(Job {
            id: row.get(0)?,
            name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
            next_execution: row.get(2)?,
            last_execution: row.get(3)?,
            interval_seconds: row.get(4)?,
            payload: row.get(5)?,
            claimed_at: row.get(6)?,
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

  pub async fn update_jobs_after_execution(&self, jobs: Vec<Job>) -> Result<()> {
    let last_execution = chrono::Utc::now().naive_utc();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        for job in jobs {
          if let Some(interval_seconds) = job.interval_seconds {
            let next_execution = last_execution
              + TimeDelta::try_seconds(interval_seconds as i64).expect("Invalid interval");
            let mut statement = tx.prepare(
              "
              UPDATE scheduler_jobs
              SET next_execution = ?, last_execution = ?, claimed_at = NULL
              WHERE id = ?
              ",
            )?;
            statement.execute(params![next_execution, last_execution, job.id])?;
          } else {
            let mut statement = tx.prepare("DELETE FROM scheduler_jobs WHERE id = ?")?;
            statement.execute([job.id])?;
          }
        }
        tx.commit()?;
        Ok::<_, rusqlite::Error>(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to update execution times");
        anyhow!("Failed to update execution times")
      })??;

    Ok(())
  }
}
