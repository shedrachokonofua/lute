use super::event_subscriber_repository::{EventSubscriberRepository, EventSubscriberStatus};
use crate::{context::ApplicationContext, scheduler::job_name::JobName};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeEventSubscriberStatusJobParameters {
  pub subscriber_id: String,
  pub status: EventSubscriberStatus,
}

pub async fn setup_event_subscriber_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobName::ChangeEventSubscriberStatus,
      Arc::new(|ctx| {
        Box::pin(async move {
          if let Some(payload) = ctx.payload {
            let params =
              serde_json::from_slice::<ChangeEventSubscriberStatusJobParameters>(&payload)?;
            let event_subscriber_repository =
              EventSubscriberRepository::new(Arc::clone(&ctx.app_context.sqlite_connection));
            event_subscriber_repository
              .set_subscriber_status(&params.subscriber_id, params.status)
              .await?;
          } else {
            error!("No payload provided for ChangeEventSubscriberStatus job, skipping");
          }
          Ok(())
        })
      }),
    )
    .await;
  Ok(())
}
