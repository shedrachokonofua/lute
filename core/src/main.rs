use core::{
  albums::album_event_subscribers::build_album_event_subscribers,
  crawler::crawler::Crawler,
  db::{build_redis_connection_pool, setup_redis_indexes},
  events::event_subscriber::EventSubscriber,
  files::file_metadata::file_name::FileName,
  helpers::fifo_queue::FifoQueue,
  lookup::lookup_event_subscribers::build_lookup_event_subscribers,
  parser::{
    parser_event_subscribers::build_parser_event_subscribers, retry::start_parser_retry_consumer,
  },
  profile::profile_event_subscribers::build_profile_event_subscribers,
  recommendations::recommendation_event_subscribers::build_recommendation_event_subscribers,
  rpc::RpcServer,
  settings::Settings,
  tracing::setup_tracing,
};
use dotenv::dotenv;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::task;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn run_rpc_server(
  settings: Settings,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  crawler: Arc<Crawler>,
  parser_retry_queue: Arc<FifoQueue<FileName>>,
) -> task::JoinHandle<()> {
  let rpc_server = RpcServer::new(settings, redis_connection_pool, crawler, parser_retry_queue);

  task::spawn(async move {
    rpc_server.run().await.unwrap();
  })
}

fn start_event_subscribers(
  settings: Settings,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  crawler: Arc<Crawler>,
) {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(build_album_event_subscribers(
    Arc::clone(&redis_connection_pool),
    settings.clone(),
    Arc::clone(&crawler.crawler_interactor),
  ));
  event_subscribers.extend(build_parser_event_subscribers(
    Arc::clone(&redis_connection_pool),
    settings.clone(),
  ));
  event_subscribers.extend(build_lookup_event_subscribers(
    Arc::clone(&redis_connection_pool),
    settings.clone(),
    Arc::clone(&crawler.crawler_interactor),
  ));
  event_subscribers.extend(build_profile_event_subscribers(
    Arc::clone(&redis_connection_pool),
    settings.clone(),
  ));
  event_subscribers.extend(build_recommendation_event_subscribers(
    Arc::clone(&redis_connection_pool),
    settings.clone(),
    Arc::clone(&crawler.crawler_interactor),
  ));
  event_subscribers.into_iter().for_each(|subscriber| {
    task::spawn(async move { subscriber.run().await });
  });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();
  let settings: Settings = Settings::new()?;
  setup_tracing(&settings.tracing)?;

  let redis_connection_pool = Arc::new(build_redis_connection_pool(settings.redis.clone()).await?);
  setup_redis_indexes(redis_connection_pool.clone()).await?;

  let parser_retry_queue: Arc<FifoQueue<FileName>> = Arc::new(FifoQueue::new(
    Arc::clone(&redis_connection_pool),
    "parser:retry",
  ));
  start_parser_retry_consumer(
    Arc::clone(&parser_retry_queue),
    Arc::clone(&redis_connection_pool),
    Arc::new(settings.clone()),
  )?;

  let crawler = Arc::new(Crawler::new(
    settings.clone(),
    Arc::clone(&redis_connection_pool),
  )?);
  crawler.run()?;

  start_event_subscribers(
    settings.clone(),
    Arc::clone(&redis_connection_pool),
    Arc::clone(&crawler),
  );

  run_rpc_server(
    settings,
    Arc::clone(&redis_connection_pool),
    Arc::clone(&crawler),
    Arc::clone(&parser_retry_queue),
  )
  .await?;

  Ok(())
}
