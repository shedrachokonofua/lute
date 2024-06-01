use super::{
  job_name::JobName,
  scheduler_repository::{Job, SchedulerRepository},
};
use crate::{
  context::ApplicationContext,
  helpers::{async_utils::ThreadSafeAsyncFn, key_value_store::KeyValueStore, priority::Priority},
  sqlite::SqliteConnection,
};
use anyhow::{anyhow, Result};
use chrono::{NaiveDateTime, TimeDelta, Utc};
use derive_builder::Builder;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
  spawn,
  sync::{mpsc::unbounded_channel, oneshot, RwLock},
  time::sleep,
};
use tracing::{error, info, instrument, warn};

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
  /**
   * If set to true, the job will be overwritten if it already exists and is not claimed
   */
  #[builder(default = "true")]
  overwrite_existing: bool,
  #[builder(default = "true")]
  skip_if_claimed: bool,
  #[builder(default, setter(strip_option))]
  payload: Option<Vec<u8>>,
  #[builder(default, setter(strip_option))]
  priority: Priority,
}

impl From<JobParameters> for Job {
  fn from(val: JobParameters) -> Self {
    Job {
      id: val.id.unwrap_or(val.name.to_string()),
      name: val.name,
      next_execution: val.next_execution,
      last_execution: None,
      interval_seconds: val.interval.map(|d| d.num_seconds() as u32),
      payload: val.payload,
      claimed_at: None,
      priority: val.priority,
      created_at: Utc::now().naive_utc(),
    }
  }
}

pub struct JobProcessorRepository {
  kv: Arc<KeyValueStore>,
}

fn processor_status_key(job_name: &JobName) -> String {
  format!("job_processor_paused:{}", job_name)
}

impl JobProcessorRepository {
  pub fn new(kv: Arc<KeyValueStore>) -> Self {
    Self { kv }
  }

  pub async fn pause(&self, job_name: &JobName, duration: Option<TimeDelta>) -> Result<()> {
    self
      .kv
      .set(
        &processor_status_key(job_name),
        1,
        duration.map(|d| d.to_std().unwrap()),
      )
      .await
  }

  pub async fn is_paused(&self, job_name: &JobName) -> Result<bool> {
    self.kv.exists(processor_status_key(job_name)).await
  }

  pub async fn resume(&self, job_name: &JobName) -> Result<()> {
    self.kv.delete(&processor_status_key(job_name)).await
  }

  pub async fn get_status(&self, job_name: &JobName) -> Result<JobProcessorStatus> {
    Ok(if self.is_paused(job_name).await? {
      JobProcessorStatus::Paused
    } else {
      JobProcessorStatus::Running
    })
  }

  pub async fn set_status(&self, job_name: &JobName, status: JobProcessorStatus) -> Result<()> {
    match status {
      JobProcessorStatus::Paused => self.pause(job_name, None).await,
      JobProcessorStatus::Running => self.resume(job_name).await,
    }
  }

  pub async fn pause_until(&self, job_name: &JobName, until: NaiveDateTime) -> Result<()> {
    self
      .pause(job_name, Some(Utc::now().naive_utc() - until))
      .await
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
    JobExecutorFn::Single($crate::helpers::async_utils::async_callback(f))
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
    JobExecutorFn::Batch($crate::helpers::async_utils::async_callback(f), $batch_size)
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
  pub cooldown: Duration,
  #[builder(setter(skip), default = "self.get_status_repo()?")]
  pub processor_repository: Arc<JobProcessorRepository>,
}

impl JobProcessorBuilder {
  fn get_status_repo(&self) -> Result<Arc<JobProcessorRepository>, String> {
    match &self.app_context {
      Some(app_context) => Ok(Arc::new(JobProcessorRepository::new(Arc::clone(
        &app_context.kv,
      )))),
      None => Err("App context is required".to_string()),
    }
  }
}

impl JobProcessor {
  fn last_execution_key(&self) -> String {
    format!("processor_last_execution:{}", self.name)
  }

  pub async fn get_last_execution(&self) -> Result<Option<NaiveDateTime>> {
    self
      .app_context
      .kv
      .get::<NaiveDateTime>(&self.last_execution_key())
      .await
  }

  #[instrument(skip_all, fields(job_name = %self.name), name = "JobProcessor::run")]
  pub async fn run(&self, scheduler_repository: Arc<SchedulerRepository>) -> Result<()> {
    let (tx, mut rx) = unbounded_channel::<oneshot::Sender<Vec<Job>>>();
    let job_name = self.name.clone();
    let claim_duration = self.claim_duration;
    let repo = Arc::clone(&scheduler_repository);
    let batch_size = self.executor.batch_size();
    spawn(async move {
      while let Some(response_channel) = rx.recv().await {
        let jobs = repo
          .claim_next_jobs(
            job_name.clone(),
            batch_size,
            TimeDelta::from_std(claim_duration)?,
          )
          .await?;
        if let Err(j) = response_channel.send(jobs) {
          error!(message = format!("{:?}", j), "Failed to send job to worker");
        }
      }
      Ok::<_, anyhow::Error>(())
    });

    for _ in 0..self.concurrency {
      let tx = tx.clone();
      let executor = self.executor.clone();
      let app_context = Arc::clone(&self.app_context);
      let cooldown = self.cooldown;
      let scheduler_repo = Arc::clone(&scheduler_repository);
      let status_repo = Arc::clone(&self.processor_repository);
      let job_name = self.name.clone();
      let last_execution_key = self.last_execution_key();

      spawn(async move {
        loop {
          match status_repo.get_status(&job_name).await {
            Ok(JobProcessorStatus::Paused) => {
              sleep(cooldown).await;
              continue;
            }
            Err(e) => {
              error!(
                message = e.to_string(),
                "Failed to get job processor status"
              );
              sleep(cooldown).await;
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

                if let Err(e) = app_context
                  .kv
                  .set::<NaiveDateTime>(&last_execution_key, Utc::now().naive_utc(), None)
                  .await
                {
                  error!(message = e.to_string(), "Failed set last execution");
                }
              }
            }
            Err(e) => {
              error!(message = e.to_string(), "Failed to receive job");
            }
          }
          sleep(cooldown).await;
        }
      });
    }
    Ok(())
  }
}

pub struct SchedulerMonitor {}
pub struct Scheduler {
  scheduler_repository: Arc<SchedulerRepository>,
  pub processor_registry: Arc<RwLock<HashMap<JobName, JobProcessor>>>,
  processor_status_repository: Arc<JobProcessorRepository>,
}

impl Scheduler {
  pub fn new(sqlite_connection: Arc<SqliteConnection>, kv: Arc<KeyValueStore>) -> Self {
    Self {
      scheduler_repository: Arc::new(SchedulerRepository::new(sqlite_connection)),
      processor_registry: Arc::new(RwLock::new(HashMap::new())),
      processor_status_repository: Arc::new(JobProcessorRepository::new(kv)),
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

  pub async fn delete_jobs_by_name(&self, job_name: JobName) -> Result<()> {
    self
      .scheduler_repository
      .delete_jobs_by_name(job_name)
      .await
  }

  pub async fn count_jobs_by_name(&self, job_name: JobName) -> Result<usize> {
    self.scheduler_repository.count_jobs_by_name(job_name).await
  }

  pub async fn get_processor_claim_duration(&self, job_name: &JobName) -> Result<TimeDelta> {
    let registry = self.processor_registry.read().await;
    let processor = registry
      .get(job_name)
      .ok_or_else(|| anyhow!("Processor not found"))?;
    let duration = TimeDelta::from_std(processor.claim_duration)?;
    Ok(duration)
  }

  pub async fn count_jobs(&self) -> Result<usize> {
    self.scheduler_repository.count_jobs().await
  }

  pub async fn count_jobs_by_each_name(&self) -> Result<HashMap<JobName, usize>> {
    self.scheduler_repository.count_jobs_by_each_name().await
  }

  pub async fn count_claimed_jobs_by_name(&self, job_name: JobName) -> Result<usize> {
    self
      .scheduler_repository
      .count_claimed_jobs_by_name(
        job_name.clone(),
        self.get_processor_claim_duration(&job_name).await?,
      )
      .await
  }

  pub async fn find_claimed_jobs_by_name(&self, job_name: JobName) -> Result<Vec<Job>> {
    self
      .scheduler_repository
      .find_claimed_jobs_by_name(
        job_name.clone(),
        self.get_processor_claim_duration(&job_name).await?,
      )
      .await
  }

  pub async fn get_processor_status(&self, job_name: &JobName) -> Result<JobProcessorStatus> {
    self.processor_status_repository.get_status(job_name).await
  }

  pub async fn set_processor_status(
    &self,
    job_name: &JobName,
    status: JobProcessorStatus,
  ) -> Result<()> {
    self
      .processor_status_repository
      .set_status(job_name, status)
      .await
  }

  pub async fn pause_processor(
    &self,
    job_name: &JobName,
    duration: Option<TimeDelta>,
  ) -> Result<()> {
    self
      .processor_status_repository
      .pause(job_name, duration)
      .await
  }

  pub async fn resume_processor(&self, job_name: &JobName) -> Result<()> {
    self.processor_status_repository.resume(job_name).await
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

  pub async fn register(&self, processor: JobProcessor) {
    self
      .processor_registry
      .write()
      .await
      .insert(processor.name.clone(), processor);
  }

  pub async fn put(&self, params: JobParameters) -> Result<bool> {
    let overwrite_existing = params.overwrite_existing;
    let skip_if_claimed = params.skip_if_claimed;
    let record: Job = params.into();
    if let Some(existing_job) = self.scheduler_repository.find_job(&record.id).await? {
      if existing_job.claimed_at.is_some() && skip_if_claimed {
        warn!(
          job_id = record.id.as_str(),
          "Job is claimed, can't schedule, skipping"
        );
        return Ok(false);
      }

      let interval_changed = match (record.interval_seconds, existing_job.interval_seconds) {
        (Some(interval_seconds), Some(existing_interval_seconds)) => {
          interval_seconds != existing_interval_seconds
        }
        _ => false,
      };
      // Force overwrite if interval has changed
      if !overwrite_existing && !interval_changed {
        info!(job_id = record.id.as_str(), "Job already exists, skipping");
        return Ok(false);
      }
    }
    self.scheduler_repository.put(record).await?;
    Ok(true)
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
