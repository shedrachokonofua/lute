use super::event_subscriber_repository::{EventSubscriberRepository, EventSubscriberStatus};
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
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeEventSubscriberStatusJobParameters {
  pub subscriber_id: String,
  pub status: EventSubscriberStatus,
}

async fn change_subscriber_status(job: Job, app_context: Arc<ApplicationContext>) -> Result<()> {
  if let Some(payload) = job.payload {
    let params = serde_json::from_slice::<ChangeEventSubscriberStatusJobParameters>(&payload)?;
    info!(
      subscriber_id = &params.subscriber_id,
      next_status = params.status.to_string(),
      "Changing event subscriber status"
    );
    let event_subscriber_repository =
      EventSubscriberRepository::new(Arc::clone(&app_context.sqlite_connection));
    event_subscriber_repository
      .set_status(&params.subscriber_id, params.status)
      .await?;
  } else {
    error!("No payload provided for ChangeEventSubscriberStatus job, skipping");
  }
  Ok(())
}

pub async fn setup_event_subscriber_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::ChangeEventSubscriberStatus)
        .executor(job_executor!(change_subscriber_status))
        .build()?,
    )
    .await;
  Ok(())
}
