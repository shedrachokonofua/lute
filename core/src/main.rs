use anyhow::Result;
use core::{
  albums::{
    album_event_subscribers::build_album_event_subscribers,
    redis_album_repository::RedisAlbumRepository,
  },
  crawler::crawler::Crawler,
  events::event_subscriber::EventSubscriber,
  files::file_metadata::file_name::FileName,
  helpers::fifo_queue::FifoQueue,
  lookup::lookup_event_subscribers::build_lookup_event_subscribers,
  parser::{
    parser_event_subscribers::build_parser_event_subscribers, retry::start_parser_retry_consumer,
  },
  profile::profile_event_subscribers::build_profile_event_subscribers,
  recommendations::recommendation_event_subscribers::build_recommendation_event_subscribers,
  redis::{build_redis_connection_pool, setup_redis_indexes},
  rpc::RpcServer,
  settings::Settings,
  sqlite::connect_to_sqlite,
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
  settings: Arc<Settings>,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
  crawler: Arc<Crawler>,
  parser_retry_queue: Arc<FifoQueue<FileName>>,
  album_read_model_repository: Arc<RedisAlbumRepository>,
) -> task::JoinHandle<()> {
  let rpc_server = RpcServer::new(
    settings,
    redis_connection_pool,
    sqlite_connection,
    crawler,
    parser_retry_queue,
    album_read_model_repository,
  );

  task::spawn(async move {
    rpc_server.run().await.unwrap();
  })
}

fn start_event_subscribers(
  settings: Arc<Settings>,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
  crawler: Arc<Crawler>,
) -> Result<()> {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(build_album_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    settings.clone(),
    Arc::clone(&crawler.crawler_interactor),
  )?);
  event_subscribers.extend(build_parser_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    settings.clone(),
  )?);
  event_subscribers.extend(build_lookup_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    settings.clone(),
    Arc::clone(&crawler.crawler_interactor),
  )?);
  event_subscribers.extend(build_profile_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    settings.clone(),
  )?);
  event_subscribers.extend(build_recommendation_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    settings,
    Arc::clone(&crawler.crawler_interactor),
  )?);
  event_subscribers.into_iter().for_each(|subscriber| {
    task::spawn(async move { subscriber.run().await });
  });
  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();
  let settings = Arc::new(Settings::new()?);
  setup_tracing(&settings.tracing)?;

  let sqlite_connection = Arc::new(connect_to_sqlite(Arc::clone(&settings)).await?);

  let redis_connection_pool = Arc::new(build_redis_connection_pool(settings.redis.clone()).await?);
  setup_redis_indexes(redis_connection_pool.clone()).await?;

  let parser_retry_queue: Arc<FifoQueue<FileName>> = Arc::new(FifoQueue::new(
    Arc::clone(&redis_connection_pool),
    "parser:retry",
  ));
  start_parser_retry_consumer(
    Arc::clone(&parser_retry_queue),
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    Arc::clone(&settings),
  )?;

  let crawler = Arc::new(Crawler::new(
    Arc::clone(&settings),
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
  )?);
  crawler.run()?;

  let album_read_model_repository = Arc::new(RedisAlbumRepository::new(Arc::clone(
    &redis_connection_pool,
  )));

  start_event_subscribers(
    Arc::clone(&settings),
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    Arc::clone(&crawler),
  )?;

  run_rpc_server(
    Arc::clone(&settings),
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    Arc::clone(&crawler),
    Arc::clone(&parser_retry_queue),
    Arc::clone(&album_read_model_repository),
  )
  .await?;

  Ok(())
}
