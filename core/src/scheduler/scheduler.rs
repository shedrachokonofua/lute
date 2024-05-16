use super::{
  job_name::JobName,
  scheduler_repository::{SchedulerJobRecord, SchedulerRepository},
};
use crate::{helpers::ThreadSafeAsyncFn, sqlite::SqliteConnection};
use anyhow::Result;
use chrono::{NaiveDateTime, TimeDelta};
use derive_builder::Builder;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{spawn, sync::Mutex, time::sleep};
use tracing::{error, info};

#[derive(Builder)]
pub struct JobParameters {
  name: JobName,
  #[builder(setter(into, strip_option))]
  id: Option<String>,
  #[builder(default = "None")]
  interval: Option<TimeDelta>,
  #[builder(default = "chrono::Utc::now().naive_utc()")]
  next_execution: NaiveDateTime,
  #[builder(default = "true")]
  overwrite_existing: bool,
  #[builder(default = "None")]
  payload: Option<Vec<u8>>,
}

impl Into<SchedulerJobRecord> for JobParameters {
  fn into(self) -> SchedulerJobRecord {
    SchedulerJobRecord {
      id: self.id.unwrap_or(self.name.to_string()),
      name: self.name,
      next_execution: self.next_execution,
      last_execution: None,
      interval_seconds: self.interval.map(|d| d.num_seconds() as u32),
      payload: self.payload,
    }
  }
}

type JobProcessor = ThreadSafeAsyncFn<Arc<SqliteConnection>>;

pub struct Scheduler {
  sqlite_connection: Arc<SqliteConnection>,
  scheduler_repository: Arc<SchedulerRepository>,
  processor_registry: Arc<Mutex<HashMap<JobName, JobProcessor>>>,
}

impl Scheduler {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self {
      sqlite_connection: Arc::clone(&sqlite_connection),
      scheduler_repository: Arc::new(SchedulerRepository::new(sqlite_connection)),
      processor_registry: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  pub async fn register(&self, job_name: JobName, processor: JobProcessor) -> () {
    self
      .processor_registry
      .lock()
      .await
      .insert(job_name, processor);
  }

  pub async fn put(&self, params: JobParameters) -> Result<()> {
    let overwrite_existing = params.overwrite_existing;
    let mut record: SchedulerJobRecord = params.into();
    if !overwrite_existing {
      if let Some(existing_record) = self.scheduler_repository.find_job(&record.id).await? {
        record.name = existing_record.name;
        record.last_execution = existing_record.last_execution;
        record.next_execution = existing_record.next_execution;
        record.interval_seconds = existing_record.interval_seconds;
        record.payload = existing_record.payload;
      }
    }
    self.scheduler_repository.put(record).await?;
    Ok(())
  }

  pub async fn run(&self) -> Result<()> {
    let scheduler_repository = Arc::clone(&self.scheduler_repository);
    let processor_registry = Arc::clone(&self.processor_registry);
    let sqlite_connection = Arc::clone(&self.sqlite_connection);

    spawn(async move {
      loop {
        match scheduler_repository.get_pending_jobs().await {
          Ok(pending_jobs) => {
            for job in pending_jobs {
              if let Some(processor) = processor_registry.lock().await.get(&job.name) {
                let processor = Arc::clone(&processor);
                let scheduler_repository = Arc::clone(&scheduler_repository);
                let sqlite_connection = Arc::clone(&sqlite_connection);
                spawn(async move {
                  match processor(Arc::clone(&sqlite_connection)).await {
                    Ok(_) => {
                      if let Err(e) = scheduler_repository
                        .update_job_after_execution(&job.id)
                        .await
                      {
                        error!(
                          message = e.to_string(),
                          "Failed to update job after execution"
                        );
                      }
                    }
                    Err(e) => {
                      error!(message = e.to_string(), "Failed to execute job");
                    }
                  }
                });
              } else {
                if job.interval_seconds.is_none() {
                  info!(
                    job_id = job.id.as_str(),
                    "Deleting transient job without handler"
                  );
                  if let Err(e) = scheduler_repository.delete_job(&job.id).await {
                    error!(message = e.to_string(), "Failed to delete job");
                  }
                }
              }
            }
          }
          Err(e) => {
            error!(message = e.to_string(), "Failed to get pending jobs");
          }
        }

        sleep(Duration::from_millis(500)).await;
      }
    });

    Ok(())
  }
}
