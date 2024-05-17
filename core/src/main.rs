use anyhow::Result;
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
  tracing::setup_tracing,
};
use mimalloc::MiMalloc;
use std::sync::Arc;
use tokio::task;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn run_rpc_server(app_context: Arc<ApplicationContext>) -> task::JoinHandle<()> {
  let rpc_server = RpcServer::new(Arc::clone(&app_context));

  task::spawn(async move {
    rpc_server.run().await.unwrap();
  })
}

fn start_event_subscribers(app_context: Arc<ApplicationContext>) -> Result<()> {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(build_album_event_subscribers(
    Arc::clone(&app_context.redis_connection_pool),
    Arc::clone(&app_context.sqlite_connection),
    app_context.settings.clone(),
    Arc::clone(&app_context.crawler.crawler_interactor),
  )?);
  event_subscribers.extend(build_parser_event_subscribers(
    Arc::clone(&app_context.redis_connection_pool),
    Arc::clone(&app_context.sqlite_connection),
    app_context.settings.clone(),
  )?);
  event_subscribers.extend(build_lookup_event_subscribers(
    Arc::clone(&app_context.redis_connection_pool),
    Arc::clone(&app_context.sqlite_connection),
    app_context.settings.clone(),
    Arc::clone(&app_context.crawler.crawler_interactor),
  )?);
  event_subscribers.extend(build_profile_event_subscribers(
    Arc::clone(&app_context.redis_connection_pool),
    Arc::clone(&app_context.sqlite_connection),
    app_context.settings.clone(),
  )?);
  event_subscribers.extend(build_recommendation_event_subscribers(
    Arc::clone(&app_context.redis_connection_pool),
    Arc::clone(&app_context.sqlite_connection),
    Arc::clone(&app_context.settings),
    Arc::clone(&app_context.crawler.crawler_interactor),
  )?);
  event_subscribers.into_iter().for_each(|subscriber| {
    task::spawn(async move { subscriber.run().await });
  });
  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let context = ApplicationContext::init().await?;
  setup_tracing(&context.settings.tracing)?;

  setup_redis_indexes(Arc::clone(&context)).await?;
  start_parser_retry_consumer(Arc::clone(&context))?;
  context.crawler.run()?;
  start_event_subscribers(Arc::clone(&context))?;
  run_rpc_server(Arc::clone(&context)).await?;

  Ok(())
}
