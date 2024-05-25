use crate::{
  context::ApplicationContext,
  crawler::crawler::CrawlJob,
  job_executor,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobParametersBuilder, JobProcessorBuilder},
    scheduler_repository::Job,
  },
};
use anyhow::{anyhow, bail, Result};
use chrono::{TimeDelta, Utc};
use std::sync::Arc;
use tokio_retry::{strategy::FibonacciBackoff, Retry};
use tracing::info;

async fn crawl(job: Job, app_context: Arc<ApplicationContext>) -> Result<()> {
  info!("Executing job, crawling");
  let crawler = Arc::clone(&app_context.crawler);
  let crawl_job: CrawlJob = job.try_into()?;

  if crawler.enforce_throttle().await? {
    bail!("Crawler is throttled");
  }

  let file_content = Retry::spawn(FibonacciBackoff::from_millis(500).take(5), || async {
    crawler.request(&crawl_job.file_name).await
  })
  .await?;

  app_context
    .file_interactor
    .put_file(&crawl_job.file_name, file_content, crawl_job.correlation_id)
    .await?;

  Ok(())
}

async fn reset_crawler_request_window(_: Job, ctx: Arc<ApplicationContext>) -> Result<()> {
  info!("Executing job, resetting crawler request window");
  ctx.crawler.reset_window_request_count().await
}

pub async fn setup_crawler_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::Crawl)
        .app_context(Arc::clone(&app_context))
        .executor(job_executor!(crawl))
        .concurrency(app_context.settings.crawler.pool_size)
        .claim_duration(
          TimeDelta::try_seconds(app_context.settings.crawler.claim_ttl_seconds as i64)
            .ok_or_else(|| anyhow!("Invalid crawler claim duration"))?
            .to_std()?,
        )
        .cooldown(
          TimeDelta::try_seconds(app_context.settings.crawler.wait_time_seconds as i64)
            .ok_or_else(|| anyhow!("Invalid crawler cooldown"))?
            .to_std()?,
        )
        .build()?,
    )
    .await;

  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::ResetCrawlerRequestWindow)
        .app_context(Arc::clone(&app_context))
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
