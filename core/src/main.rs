use anyhow::Result;
use core::{
  albums::album_event_subscribers::build_album_event_subscribers,
  context::ApplicationContext,
  crawler::crawler_jobs::setup_crawler_jobs,
  events::{event_subscriber::EventSubscriber, event_subscriber_jobs::setup_event_subscriber_jobs},
  helpers::key_value_store::setup_kv_jobs,
  lookup::lookup_event_subscribers::build_lookup_event_subscribers,
  parser::{
    parser_event_subscribers::build_parser_event_subscribers, retry::start_parser_retry_consumer,
  },
  profile::profile_event_subscribers::build_profile_event_subscribers,
  recommendations::recommendation_event_subscribers::build_recommendation_event_subscribers,
  redis::setup_redis_indexes,
  rpc::RpcServer,
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

async fn setup_jobs(context: Arc<ApplicationContext>) -> Result<()> {
  setup_crawler_jobs(Arc::clone(&context)).await?;
  setup_event_subscriber_jobs(Arc::clone(&context)).await?;
  setup_kv_jobs(Arc::clone(&context)).await?;
  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let context = ApplicationContext::init().await?;
  setup_redis_indexes(Arc::clone(&context)).await?;
  start_parser_retry_consumer(Arc::clone(&context))?;
  start_event_subscribers(Arc::clone(&context))?;
  setup_jobs(Arc::clone(&context)).await?;
  context.scheduler.run(Arc::clone(&context)).await?;
  context.crawler.run()?;
  RpcServer::new(context).run().await?;

  Ok(())
}
