use super::{job_name::JobName, scheduler::JobParametersBuilder, scheduler_repository::Job};
use crate::{context::ApplicationContext, proto};
use chrono::{NaiveDateTime, TimeDelta};
use std::{str::FromStr, sync::Arc};
use tonic::{async_trait, Request, Response, Status};

impl Into<proto::Job> for Job {
  fn into(self) -> proto::Job {
    proto::Job {
      id: self.id,
      name: self.name.to_string(),
      next_execution: self.next_execution.to_string(),
      last_execution: self.last_execution.map(|d| d.to_string()),
      interval_seconds: self.interval_seconds,
      payload: self.payload,
      claimed_at: self.claimed_at.map(|d| d.to_string()),
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
  async fn get_registered_processors(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetRegisteredProcessorsReply>, Status> {
    let processors = self.app_context.scheduler.get_registered_processors().await;
    Ok(Response::new(proto::GetRegisteredProcessorsReply {
      processors: processors.into_iter().map(|p| p.to_string()).collect(),
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
      .name(JobName::from_str(&params.name).map_err(|e| Status::invalid_argument(e.to_string()))?)
      .payload(params.payload);

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
}
