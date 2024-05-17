use anyhow::Result;
use chrono::TimeDelta;
use core::{
  albums::album_event_subscribers::build_album_event_subscribers,
  context::ApplicationContext,
  events::event_subscriber::EventSubscriber,
  lookup::lookup_event_subscribers::build_lookup_event_subscribers,
  parser::{
    parser_event_subscribers::build_parser_event_subscribers, retry::start_parser_retry_consumer,
  },
  profile::profile_event_subscribers::build_profile_event_subscribers,
  recommendations::recommendation_event_subscribers::build_recommendation_event_subscribers,
  redis::setup_redis_indexes,
  rpc::RpcServer,
  scheduler::{
    job_name::JobName,
    scheduler::{JobParametersBuilder, Scheduler},
  },
};
use mimalloc::MiMalloc;
use std::sync::Arc;
use tokio::task;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn start_event_subscribers(app_context: Arc<ApplicationContext>) -> Result<()> {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(build_album_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_parser_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_lookup_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_profile_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_recommendation_event_subscribers(Arc::clone(
    &app_context,
  ))?);
  event_subscribers.into_iter().for_each(|subscriber| {
    task::spawn(async move { subscriber.run().await });
  });
  Ok(())
}

async fn setup_jobs(scheduler: Arc<Scheduler>) -> Result<()> {
  scheduler
    .register(
      JobName::HelloWorld,
      Arc::new(|_| {
        Box::pin(async move {
          println!(
            "Hello, world! {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
          );
          Ok(())
        })
      }),
    )
    .await;

  scheduler
    .put(
      JobParametersBuilder::default()
        .name(JobName::HelloWorld)
        .interval(TimeDelta::try_seconds(15))
        .build()?,
    )
    .await?;

  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let context = ApplicationContext::init().await?;
  setup_redis_indexes(Arc::clone(&context)).await?;
  start_parser_retry_consumer(Arc::clone(&context))?;
  start_event_subscribers(Arc::clone(&context))?;
  setup_jobs(Arc::clone(&context.scheduler)).await?;
  context.scheduler.run(Arc::clone(&context)).await?;
  context.crawler.run()?;
  RpcServer::new(context).run().await?;

  Ok(())
}
