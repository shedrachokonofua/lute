use super::job_name::JobName;
use crate::{helpers::priority::Priority, sqlite::SqliteConnection};
use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveDateTime, TimeDelta, Utc};
use rusqlite::{params, types::Value};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, rc::Rc, str::FromStr, sync::Arc};
use tracing::{error, instrument};

#[derive(Clone)]
pub struct SchedulerRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

#[derive(Debug, Clone)]
pub struct Job {
  pub id: String,
  pub name: JobName,
  pub created_at: NaiveDateTime,
  pub next_execution: NaiveDateTime,
  pub last_execution: Option<NaiveDateTime>,
  pub interval_seconds: Option<u32>,
  pub payload: Option<Vec<u8>>,
  pub claimed_at: Option<NaiveDateTime>,
  pub priority: Priority,
}

impl Job {
  pub fn payload<T: DeserializeOwned>(&self) -> Result<T> {
    self
      .payload
      .as_ref()
      .map(|p| serde_json::from_slice::<T>(p))
      .transpose()?
      .ok_or_else(|| anyhow!("Failed to get payload"))
  }
}

impl SchedulerRepository {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(skip(self), name = "SchedulerRepository::put")]
  pub async fn put(&self, record: Job) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          INSERT INTO scheduler_jobs (
            id, 
            name, 
            next_execution, 
            last_execution, 
            interval_seconds, 
            payload, 
            priority,
            created_at
          )
          VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
          ON CONFLICT (id) DO UPDATE SET 
            name = excluded.name,
            next_execution = excluded.next_execution, 
            last_execution = excluded.last_execution, 
            interval_seconds = excluded.interval_seconds,
            payload = excluded.payload,
            priority = excluded.priority,
            created_at = excluded.created_at
          ",
        )?;
        statement.execute(params![
          record.id,
          record.name.to_string(),
          record.next_execution,
          record.last_execution,
          record.interval_seconds,
          record.payload,
          record.priority as u32
        ])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set cursor");
        anyhow!("Failed to set cursor")
      })?
  }

  #[instrument(skip_all, name = "SchedulerRepository::put_many", fields(count = records.len()))]
  pub async fn put_many(&self, records: Vec<Job>) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        {
          let mut statement = tx.prepare(
            "
            INSERT INTO scheduler_jobs (
              id, 
              name, 
              next_execution, 
              last_execution, 
              interval_seconds, 
              payload, 
              priority,
              created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT (id) DO UPDATE SET 
              name = excluded.name,
              next_execution = excluded.next_execution, 
              last_execution = excluded.last_execution, 
              interval_seconds = excluded.interval_seconds,
              payload = excluded.payload,
              priority = excluded.priority,
              created_at = excluded.created_at
            ",
          )?;
          for record in records {
            statement.execute(params![
              record.id,
              record.name.to_string(),
              record.next_execution,
              record.last_execution,
              record.interval_seconds,
              record.payload,
              record.priority as u32
            ])?;
          }
        }
        tx.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to set cursor");
        anyhow!("Failed to set cursor")
      })?
  }

  #[instrument(skip(self), name = "SchedulerRepository::get_jobs")]
  pub async fn get_jobs(&self) -> Result<Vec<Job>> {
    let jobs = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT 
            id, 
            name, 
            next_execution, 
            last_execution, 
            interval_seconds, 
            payload, 
            claimed_at, 
            priority, 
            created_at
          FROM scheduler_jobs
          ",
        )?;
        let rows = statement
          .query_map([], |row| {
            let result = JobName::from_str(row.get::<_, String>(1)?.as_str())
              .inspect_err(|e| {
                error!(message = e.to_string(), "Failed to get job name");
              })
              .ok()
              .map(|name| {
                let job = Job {
                  id: row.get(0)?,
                  name,
                  next_execution: row.get(2)?,
                  last_execution: row.get(3)?,
                  interval_seconds: row.get(4)?,
                  payload: row.get(5)?,
                  claimed_at: row.get(6)?,
                  priority: Priority::try_from(row.get::<_, u32>(7)?).unwrap(),
                  created_at: row.get(8)?,
                };
                Ok::<_, rusqlite::Error>(job)
              })
              .transpose()?;
            Ok(result)
          })?
          .collect::<Result<Vec<_>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get all jobs");
        anyhow!("Failed to get all jobs")
      })??;

    Ok(jobs.into_iter().flatten().collect())
  }

  #[instrument(skip(self), name = "SchedulerRepository::set_many_claimed_at")]
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

  #[instrument(skip(self), name = "SchedulerRepository::peek_next_jobs")]
  pub async fn peek_next_jobs(
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
          SELECT 
            id, 
            name, 
            next_execution, 
            last_execution, 
            interval_seconds, 
            payload, 
            claimed_at, 
            priority, 
            created_at
          FROM scheduler_jobs
          WHERE
            name = ?
            AND next_execution <= datetime('now')
            AND (
              claimed_at IS NULL
              OR claimed_at < datetime(?)
            )
          ORDER BY priority, next_execution, id
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
                priority: Priority::try_from(row.get::<_, u32>(7)?).unwrap(),
                created_at: row.get(8)?,
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

    Ok(jobs)
  }

  #[instrument(skip(self), name = "SchedulerRepository::claim_next_jobs")]
  pub async fn claim_next_jobs(
    &self,
    job_name: JobName,
    count: u32,
    claim_duration: Duration,
  ) -> Result<Vec<Job>> {
    let jobs = self.peek_next_jobs(job_name, count, claim_duration).await?;

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

  #[instrument(skip(self), name = "SchedulerRepository::count_jobs_by_name")]
  pub async fn count_jobs_by_name(&self, job_name: JobName) -> Result<usize> {
    let count = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        conn.query_row(
          "
          SELECT COUNT(*)
          FROM scheduler_jobs
          WHERE name = ?
          ",
          [job_name.to_string()],
          |row| row.get::<_, usize>(0),
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to count jobs by name");
        anyhow!("Failed to count jobs by name")
      })??;

    Ok(count)
  }

  #[instrument(skip(self), name = "SchedulerRepository::count_jobs_by_each_name")]
  pub async fn count_jobs_by_each_name(&self) -> Result<HashMap<JobName, usize>> {
    let results = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT name, COUNT(*)
          FROM scheduler_jobs
          GROUP BY name
          ",
        )?;
        let rows = statement
          .query_map([], |row| {
            if let Ok(name) = JobName::from_str(row.get::<_, String>(0)?.as_str()) {
              let count = row.get::<_, usize>(1)?;
              Ok(Some((name, count)))
            } else {
              Ok(None)
            }
          })?
          .collect::<Result<Vec<Option<(_, _)>>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to count jobs by each name");
        anyhow!("Failed to count jobs by each name: {:?}", e.to_string())
      })??;

    let mut counts = HashMap::new();
    for (name, count) in results.into_iter().flatten() {
      counts.insert(name, count);
    }

    Ok(counts)
  }

  #[instrument(skip(self), name = "SchedulerRepository::count_claimed_jobs_by_name")]
  pub async fn count_claimed_jobs_by_name(
    &self,
    job_name: JobName,
    claim_duration: Duration,
  ) -> Result<usize> {
    let oldest_claimed_at = Utc::now().naive_utc() - claim_duration;
    let count = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        conn.query_row(
          "
          SELECT COUNT(*)
          FROM scheduler_jobs
          WHERE 
            name = ? 
            AND claimed_at IS NOT NULL
            AND claimed_at >= datetime(?)
          ",
          params![job_name.to_string(), oldest_claimed_at],
          |row| row.get::<_, usize>(0),
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to count claimed jobs");
        anyhow!("Failed to count claimed jobs: {:?}", e.to_string())
      })??;

    Ok(count)
  }

  #[instrument(skip(self), name = "SchedulerRepository::count_jobs")]
  pub async fn count_jobs(&self) -> Result<usize> {
    let count = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        conn.query_row("SELECT COUNT(*) FROM scheduler_jobs", [], |row| {
          row.get::<_, usize>(0)
        })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to count jobs");
        anyhow!("Failed to count jobs: {:?}", e.to_string())
      })??;

    Ok(count)
  }

  #[instrument(skip(self), name = "SchedulerRepository::find_claimed_jobs_by_name")]
  pub async fn find_claimed_jobs_by_name(
    &self,
    job_name: JobName,
    claim_duration: Duration,
  ) -> Result<Vec<Job>> {
    let oldest_claimed_at = Utc::now().naive_utc() - claim_duration;
    let jobs = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT
            id, 
            name, 
            next_execution, 
            last_execution, 
            interval_seconds, 
            payload, 
            claimed_at, 
            priority, 
            created_at
          FROM scheduler_jobs
          WHERE 
            name = ? 
            AND claimed_at IS NOT NULL
            AND claimed_at >= datetime(?)
          ",
        )?;
        let rows = statement
          .query_map(params![job_name.to_string(), oldest_claimed_at], |row| {
            Ok(Job {
              id: row.get(0)?,
              name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
              next_execution: row.get(2)?,
              last_execution: row.get(3)?,
              interval_seconds: row.get(4)?,
              payload: row.get(5)?,
              claimed_at: row.get(6)?,
              priority: Priority::try_from(row.get::<_, u32>(7)?).unwrap(),
              created_at: row.get(8)?,
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

  #[instrument(skip_all, name = "SchedulerRepository::find_jobs", fields(job_ids = job_ids.len()))]
  pub async fn find_jobs(&self, job_ids: Vec<String>) -> Result<HashMap<String, Job>> {
    let ids = job_ids.into_iter().map(Value::from).collect::<Vec<_>>();
    let jobs = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare(
          "
          SELECT
            id, 
            name, 
            next_execution, 
            last_execution, 
            interval_seconds, 
            payload, 
            claimed_at, 
            priority, 
            created_at
          FROM scheduler_jobs
          WHERE id IN rarray(?)
          ",
        )?;
        let rows = statement
          .query_map(params![Rc::new(ids)], |row| {
            let id = row.get::<_, String>(0)?;
            Ok((
              id.clone(),
              Job {
                id,
                name: JobName::from_str(row.get::<_, String>(1)?.as_str()).unwrap(),
                next_execution: row.get(2)?,
                last_execution: row.get(3)?,
                interval_seconds: row.get(4)?,
                payload: row.get(5)?,
                claimed_at: row.get(6)?,
                priority: Priority::try_from(row.get::<_, u32>(7)?).unwrap(),
                created_at: row.get(8)?,
              },
            ))
          })?
          .collect::<Result<HashMap<_, _>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to claim next job");
        anyhow!("Failed to claim next job")
      })??;

    Ok(jobs)
  }

  #[instrument(skip(self), name = "SchedulerRepository::find_job")]
  pub async fn find_job(&self, job_id: &str) -> Result<Option<Job>> {
    self
      .find_jobs(vec![job_id.to_string()])
      .await
      .map(|mut jobs| jobs.remove(job_id))
  }

  #[instrument(skip(self), name = "SchedulerRepository::delete_job")]
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

  #[instrument(skip(self), name = "SchedulerRepository::delete_all_jobs")]
  pub async fn delete_all_jobs(&self) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare("DELETE FROM scheduler_jobs")?;
        statement.execute([])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete all jobs");
        anyhow!("Failed to delete all jobs")
      })?
  }

  #[instrument(skip(self), name = "SchedulerRepository::delete_jobs_by_name")]
  pub async fn delete_jobs_by_name(&self, job_name: JobName) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let mut statement = conn.prepare("DELETE FROM scheduler_jobs WHERE name = ?")?;
        statement.execute([job_name.to_string()])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete jobs by name");
        anyhow!("Failed to delete jobs by name")
      })?
  }

  #[instrument(skip(self), name = "SchedulerRepository::update_execution_times")]
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

  #[instrument(skip(self), name = "SchedulerRepository::update_jobs_after_execution")]
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
