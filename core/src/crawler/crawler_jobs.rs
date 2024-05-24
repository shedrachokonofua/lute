use crate::{
  context::ApplicationContext,
  job_executor,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobParametersBuilder, JobProcessorBuilder},
    scheduler_repository::Job,
  },
};
use anyhow::Result;
use chrono::{TimeDelta, Utc};
use std::sync::Arc;
use tracing::info;

async fn reset_crawler_request_window(_: Job, ctx: Arc<ApplicationContext>) -> Result<()> {
  info!("Executing job, resetting crawler request window");
  ctx
    .crawler
    .crawler_interactor
    .reset_window_request_count()
    .await
}

pub async fn setup_crawler_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::ResetCrawlerRequestWindow)
        .executor(job_executor!(reset_crawler_request_window))
        .build()?,
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
        .overwrite_existing(false)
        .build()?,
    )
    .await?;
  Ok(())
}
