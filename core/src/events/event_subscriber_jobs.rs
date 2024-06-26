use super::event_repository::{EventRepository, EventSubscriberStatus};
use crate::{
  context::ApplicationContext,
  job_executor,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobProcessorBuilder},
    scheduler_repository::Job,
  },
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeEventSubscriberStatusJobParameters {
  pub subscriber_id: String,
  pub status: EventSubscriberStatus,
}

async fn change_subscriber_status(job: Job, app_context: Arc<ApplicationContext>) -> Result<()> {
  let params = job.payload::<ChangeEventSubscriberStatusJobParameters>()?;
  info!(
    subscriber_id = &params.subscriber_id,
    next_status = params.status.to_string(),
    "Changing event subscriber status"
  );
  let event_repository = EventRepository::new(Arc::clone(&app_context.sqlite_connection));
  event_repository
    .set_subscriber_status(&params.subscriber_id, params.status)
    .await?;
  Ok(())
}

pub async fn setup_event_subscriber_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::ChangeEventSubscriberStatus)
        .app_context(Arc::clone(&app_context))
        .executor(job_executor!(change_subscriber_status))
        .build()?,
    )
    .await;
  Ok(())
}
