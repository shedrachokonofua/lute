use crate::{
  context::ApplicationContext,
  scheduler::{job_name::JobName, scheduler::JobParametersBuilder},
};
use anyhow::Result;
use chrono::{TimeDelta, Utc};
use std::sync::Arc;
use tracing::info;

pub async fn setup_crawler_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobName::ResetCrawlerRequestWindow,
      Arc::new(|ctx| {
        Box::pin(async move {
          info!("Executing job, resetting crawler request window");
          ctx
            .app_context
            .crawler
            .crawler_interactor
            .reset_window_request_count()
            .await
        })
      }),
    )
    .await;

  let window =
    TimeDelta::try_seconds(app_context.settings.crawler.rate_limit.window_seconds as i64).unwrap();
  app_context
    .scheduler
    .put(
      JobParametersBuilder::default()
        .name(JobName::ResetCrawlerRequestWindow)
        .interval(window)
        .next_execution(Utc::now().naive_utc() + window)
        .build()?,
    )
    .await?;
  Ok(())
}
