use super::{
  job_name::JobName,
  scheduler_repository::{Job, SchedulerRepository},
};
use crate::{
  context::ApplicationContext,
  helpers::{async_utils::ThreadSafeAsyncFn, key_value_store::KeyValueStore, priority::Priority},
  sqlite::SqliteConnection,
};
use anyhow::Result;
use chrono::{NaiveDateTime, TimeDelta, Utc};
use derive_builder::Builder;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
  spawn,
  sync::{mpsc::unbounded_channel, oneshot, RwLock},
  time::sleep,
};
use tracing::{error, info};

pub enum JobProcessorStatus {
  Running,
  Paused,
}

#[derive(Builder)]
pub struct JobParameters {
  name: JobName,
  #[builder(default, setter(into))]
  id: Option<String>,
  #[builder(default, setter(strip_option))]
  interval: Option<TimeDelta>,
  #[builder(default = "chrono::Utc::now().naive_utc()")]
  next_execution: NaiveDateTime,
  #[builder(default = "true")]
  overwrite_existing: bool,
  #[builder(default)]
  payload: Option<Vec<u8>>,
  #[builder(default)]
  priority: Priority,
}

impl Into<Job> for JobParameters {
  fn into(self) -> Job {
    Job {
      id: self.id.unwrap_or(self.name.to_string()),
      name: self.name,
      next_execution: self.next_execution,
      last_execution: None,
      interval_seconds: self.interval.map(|d| d.num_seconds() as u32),
      payload: self.payload,
      claimed_at: None,
      priority: self.priority,
    }
  }
}

pub struct JobProcessorStatusRepository {
  kv: Arc<KeyValueStore>,
}

impl JobProcessorStatusRepository {
  pub fn new(kv: Arc<KeyValueStore>) -> Self {
    Self { kv }
  }

  pub async fn get(&self, job_name: &JobName) -> Result<JobProcessorStatus> {
    match self
      .kv
      .exists(format!("job_processor_paused:{}", job_name.to_string()))
      .await?
    {
      true => Ok(JobProcessorStatus::Paused),
      false => Ok(JobProcessorStatus::Running),
    }
  }

  pub async fn set(&self, job_name: &JobName, status: JobProcessorStatus) -> Result<()> {
    let key = format!("job_processor_paused:{}", job_name.to_string());
    match status {
      JobProcessorStatus::Paused => self.kv.set(&key, 1, None).await,
      JobProcessorStatus::Running => self.kv.delete(&key).await,
    }
  }

  pub async fn pause_until(&self, job_name: &JobName, until: NaiveDateTime) -> Result<()> {
    self
      .pause(job_name, Some(Utc::now().naive_utc() - until))
      .await
  }

  pub async fn pause(&self, job_name: &JobName, duration: Option<TimeDelta>) -> Result<()> {
    self
      .kv
      .set(
        &job_name.to_string(),
        1,
        duration.map(|d| d.to_std().unwrap()),
      )
      .await
  }

  pub async fn is_paused(&self, job_name: &JobName) -> Result<bool> {
    self
      .kv
      .exists(format!("job_processor_paused:{}", job_name.to_string()))
      .await
  }

  pub async fn resume(&self, job_name: &JobName) -> Result<()> {
    self.kv.delete(&job_name.to_string()).await
  }
}

#[derive(Clone)]
pub enum JobExecutorFn {
  Single(ThreadSafeAsyncFn<(Job, Arc<ApplicationContext>)>),
  Batch(ThreadSafeAsyncFn<(Vec<Job>, Arc<ApplicationContext>)>, u32),
}

#[macro_export]
macro_rules! job_executor {
  ($f: expr) => {{
    fn f(
      (job, app_context): (Job, Arc<ApplicationContext>),
    ) -> impl futures::Future<Output = Result<(), anyhow::Error>> + Send + 'static {
      $f(job, app_context)
    }
    JobExecutorFn::Single(crate::helpers::async_utils::async_callback(f))
  }};
}

#[macro_export]
macro_rules! batch_job_executor {
  ($f: expr, $batch_size: expr) => {{
    fn f(
      (jobs, app_context): (Vec<Job>, Arc<ApplicationContext>),
    ) -> impl futures::Future<Output = Result<(), anyhow::Error>> + Send + 'static {
      $f(jobs, app_context)
    }
    JobExecutorFn::Batch(crate::helpers::async_utils::async_callback(f), $batch_size)
  }};
}

impl JobExecutorFn {
  pub fn batch_size(&self) -> u32 {
    match self {
      JobExecutorFn::Single(_) => 1,
      JobExecutorFn::Batch(_, size) => *size,
    }
  }

  async fn execute(&self, mut jobs: Vec<Job>, app_context: Arc<ApplicationContext>) -> Result<()> {
    if jobs.is_empty() {
      return Ok(());
    }

    match self {
      JobExecutorFn::Single(f) => f((jobs.pop().unwrap(), app_context)).await,
      JobExecutorFn::Batch(f, _) => f((jobs, app_context)).await,
    }
  }
}

#[derive(Builder)]
pub struct JobProcessor {
  pub name: JobName,
  pub app_context: Arc<ApplicationContext>,
  pub executor: JobExecutorFn,
  #[builder(default = "1")]
  pub concurrency: u32,
  #[builder(default = "Duration::from_secs(60)")]
  pub claim_duration: Duration,
  #[builder(default = "Duration::from_secs(1)")]
  pub heartbeat: Duration,
  #[builder(setter(skip), default = "self.get_status_repo()?")]
  pub status_repository: Arc<JobProcessorStatusRepository>,
}

impl JobProcessorBuilder {
  fn get_status_repo(&self) -> Result<Arc<JobProcessorStatusRepository>, String> {
    match &self.app_context {
      Some(app_context) => Ok(Arc::new(JobProcessorStatusRepository::new(Arc::clone(
        &app_context.kv,
      )))),
      None => Err("App context is required".to_string()),
    }
  }
}

impl JobProcessor {
  pub async fn run(&self, scheduler_repository: Arc<SchedulerRepository>) -> Result<()> {
    let (tx, mut rx) = unbounded_channel::<oneshot::Sender<Vec<Job>>>();
    let job_name = self.name.clone();
    let claim_duration = self.claim_duration.clone();
    let repo = Arc::clone(&scheduler_repository);
    let batch_size = self.executor.batch_size();
    spawn(async move {
      while let Some(response_channel) = rx.recv().await {
        let job = repo
          .claim_next_jobs(
            job_name.clone(),
            batch_size,
            TimeDelta::from_std(claim_duration)?,
          )
          .await?;
        if let Err(j) = response_channel.send(job) {
          error!(message = format!("{:?}", j), "Failed to send job to worker");
        }
      }
      Ok::<_, anyhow::Error>(())
    });

    for _ in 0..self.concurrency {
      let tx = tx.clone();
      let executor = self.executor.clone();
      let app_context = Arc::clone(&self.app_context);
      let heartbeat = self.heartbeat;
      let scheduler_repo = Arc::clone(&scheduler_repository);
      let status_repo = Arc::clone(&self.status_repository);
      let job_name = self.name.clone();

      spawn(async move {
        loop {
          match status_repo.get(&job_name).await {
            Ok(JobProcessorStatus::Paused) => {
              sleep(heartbeat).await;
              continue;
            }
            Err(e) => {
              error!(
                message = e.to_string(),
                "Failed to get job processor status"
              );
              sleep(heartbeat).await;
              continue;
            }
            _ => {}
          }

          let (job_sender, job_receiver) = oneshot::channel();
          if let Err(e) = tx.send(job_sender) {
            error!(message = format!("{:?}", e), "Failed to send claim request");
          }
          match job_receiver.await {
            Ok(jobs) => {
              if !jobs.is_empty() {
                if let Err(e) = executor
                  .execute(jobs.clone(), Arc::clone(&app_context))
                  .await
                {
                  error!(message = e.to_string(), "Failed to execute job");
                }

                if let Err(e) = scheduler_repo.update_jobs_after_execution(jobs).await {
                  error!(
                    message = e.to_string(),
                    "Failed to update jobs after execution"
                  );
                }
              }
            }
            Err(e) => {
              error!(message = e.to_string(), "Failed to receive job");
            }
          }
          sleep(heartbeat).await;
        }
      });
    }
    Ok(())
  }
}

pub struct Scheduler {
  scheduler_repository: Arc<SchedulerRepository>,
  pub processor_registry: Arc<RwLock<HashMap<JobName, JobProcessor>>>,
  processor_status_repository: Arc<JobProcessorStatusRepository>,
}

impl Scheduler {
  pub fn new(sqlite_connection: Arc<SqliteConnection>, kv: Arc<KeyValueStore>) -> Self {
    Self {
      scheduler_repository: Arc::new(SchedulerRepository::new(sqlite_connection)),
      processor_registry: Arc::new(RwLock::new(HashMap::new())),
      processor_status_repository: Arc::new(JobProcessorStatusRepository::new(kv)),
    }
  }

  pub async fn get_jobs(&self) -> Result<Vec<Job>> {
    self.scheduler_repository.get_jobs().await
  }

  pub async fn delete_job(&self, job_id: &str) -> Result<()> {
    self.scheduler_repository.delete_job(job_id).await
  }

  pub async fn delete_all_jobs(&self) -> Result<()> {
    self.scheduler_repository.delete_all_jobs().await
  }

  pub async fn get_processor_status(&self, job_name: &JobName) -> Result<JobProcessorStatus> {
    self.processor_status_repository.get(job_name).await
  }

  pub async fn set_processor_status(
    &self,
    job_name: &JobName,
    status: JobProcessorStatus,
  ) -> Result<()> {
    self.processor_status_repository.set(job_name, status).await
  }

  pub async fn get_registered_processors(&self) -> Vec<JobName> {
    self
      .processor_registry
      .read()
      .await
      .keys()
      .cloned()
      .collect()
  }

  pub async fn register(&self, processor: JobProcessor) -> () {
    self
      .processor_registry
      .write()
      .await
      .insert(processor.name.clone(), processor);
  }

  pub async fn put(&self, params: JobParameters) -> Result<()> {
    let overwrite_existing = params.overwrite_existing;
    let record: Job = params.into();
    if let Some(existing_job) = self.scheduler_repository.find_job(&record.id).await? {
      let interval_changed = match (record.interval_seconds, existing_job.interval_seconds) {
        (Some(interval_seconds), Some(existing_interval_seconds)) => {
          interval_seconds != existing_interval_seconds
        }
        _ => false,
      };
      // Force overwrite if interval has changed
      if !overwrite_existing && !interval_changed {
        info!(job_id = record.id.as_str(), "Job already exists, skipping");
        return Ok(());
      }
    }
    self.scheduler_repository.put(record).await?;
    Ok(())
  }

  pub async fn run(&self) -> Result<()> {
    let processor_registry = Arc::clone(&self.processor_registry);

    for processor in processor_registry.read().await.values() {
      processor
        .run(Arc::clone(&self.scheduler_repository))
        .await?;
    }

    Ok(())
  }
}
