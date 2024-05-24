use super::{
  job_name::JobName,
  scheduler::{JobParametersBuilder, JobProcessorStatus},
  scheduler_repository::Job,
};
use crate::{context::ApplicationContext, proto};
use chrono::{NaiveDateTime, TimeDelta};
use futures::future::try_join_all;
use std::{str::FromStr, sync::Arc};
use tonic::{async_trait, Request, Response, Status};

impl From<Job> for proto::Job {
  fn from(val: Job) -> Self {
    proto::Job {
      id: val.id,
      name: val.name.to_string(),
      next_execution: val.next_execution.to_string(),
      last_execution: val.last_execution.map(|d| d.to_string()),
      interval_seconds: val.interval_seconds,
      payload: val.payload,
      claimed_at: val.claimed_at.map(|d| d.to_string()),
      priority: val.priority as i32,
    }
  }
}

impl From<JobProcessorStatus> for i32 {
  fn from(val: JobProcessorStatus) -> Self {
    match val {
      JobProcessorStatus::Running => proto::JobProcessorStatus::ProcessorRunning as i32,
      JobProcessorStatus::Paused => proto::JobProcessorStatus::ProcessorPaused as i32,
    }
  }
}

impl From<i32> for JobProcessorStatus {
  fn from(value: i32) -> Self {
    match value {
      x if x == proto::JobProcessorStatus::ProcessorRunning as i32 => JobProcessorStatus::Running,
      x if x == proto::JobProcessorStatus::ProcessorPaused as i32 => JobProcessorStatus::Paused,
      _ => panic!("Invalid JobProcessorStatus value"),
    }
  }
}

pub struct SchedulerService {
  app_context: Arc<ApplicationContext>,
}

impl SchedulerService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self { app_context }
  }
}

#[async_trait]
impl proto::SchedulerService for SchedulerService {
  async fn set_job_processor_status(
    &self,
    request: Request<proto::SetProcessorStatusRequest>,
  ) -> Result<Response<()>, Status> {
    let params = request.into_inner();
    let job_name =
      JobName::from_str(&params.name).map_err(|e| Status::invalid_argument(e.to_string()))?;
    self
      .app_context
      .scheduler
      .set_processor_status(&job_name, params.status.into())
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(()))
  }

  async fn get_registered_processors(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetRegisteredProcessorsReply>, Status> {
    let registered_processors = self.app_context.scheduler.processor_registry.read().await;
    let registered_processors = registered_processors.values().collect::<Vec<_>>();
    let statuses = try_join_all(
      registered_processors
        .iter()
        .map(|j| self.app_context.scheduler.get_processor_status(&j.name)),
    )
    .await
    .map_err(|e| Status::internal(e.to_string()))?;

    let processors = registered_processors
      .into_iter()
      .zip(statuses)
      .map(|(processor, status)| proto::JobProcessor {
        job_name: processor.name.to_string(),
        status: status.into(),
        claim_duration_seconds: processor.claim_duration.as_secs(),
        concurrency: processor.concurrency,
        cooldown_seconds: processor.cooldown.as_secs(),
      })
      .collect::<Vec<_>>();

    Ok(Response::new(proto::GetRegisteredProcessorsReply {
      processors,
    }))
  }

  async fn get_jobs(&self, _request: Request<()>) -> Result<Response<proto::GetJobsReply>, Status> {
    let jobs = self
      .app_context
      .scheduler
      .get_jobs()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    Ok(Response::new(proto::GetJobsReply {
      jobs: jobs.into_iter().map(|j| j.into()).collect(),
    }))
  }

  async fn put_job(&self, request: Request<proto::PutJobRequest>) -> Result<Response<()>, Status> {
    let params = request.into_inner();
    let mut builder = JobParametersBuilder::default();
    builder
      .id(params.id)
      .name(JobName::from_str(&params.name).map_err(|e| Status::invalid_argument(e.to_string()))?);

    if let Some(payload) = params.payload {
      builder.payload(payload);
    }

    if let Some(next_execution) = params.next_execution {
      builder.next_execution(
        NaiveDateTime::parse_from_str(&next_execution, "%Y-%m-%dT%H:%M:%S")
          .map_err(|e| Status::invalid_argument(e.to_string()))?,
      );
    }

    if let Some(interval) = params.interval_seconds {
      builder.interval(TimeDelta::try_seconds(interval as i64).unwrap());
    }

    self
      .app_context
      .scheduler
      .put(
        builder
          .build()
          .map_err(|e| Status::internal(e.to_string()))?,
      )
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(()))
  }

  async fn delete_job(
    &self,
    request: Request<proto::DeleteJobRequest>,
  ) -> Result<Response<()>, Status> {
    self
      .app_context
      .scheduler
      .delete_job(&request.into_inner().id)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(()))
  }

  async fn delete_all_jobs(&self, _request: Request<()>) -> Result<Response<()>, Status> {
    self
      .app_context
      .scheduler
      .delete_all_jobs()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(()))
  }
}
