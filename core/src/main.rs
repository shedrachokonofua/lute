use anyhow::Result;
use core::{
  albums::{
    album_event_subscribers::build_album_event_subscribers,
    redis_album_search_index::RedisAlbumSearchIndex,
    sqlite_album_repository::SqliteAlbumRepository,
  },
  crawler::{crawler::Crawler, crawler_interactor::CrawlerInteractor},
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
use mimalloc::MiMalloc;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tokio::task;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn run_rpc_server(
  settings: Arc<Settings>,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
  crawler_interactor: Arc<CrawlerInteractor>,
  parser_retry_queue: Arc<FifoQueue<FileName>>,
  album_repository: Arc<SqliteAlbumRepository>,
  album_search_index: Arc<RedisAlbumSearchIndex>,
) -> task::JoinHandle<()> {
  let rpc_server = RpcServer::new(
    settings,
    redis_connection_pool,
    sqlite_connection,
    crawler_interactor,
    parser_retry_queue,
    album_repository,
    album_search_index,
  );

  task::spawn(async move {
    rpc_server.run().await.unwrap();
  })
}

fn start_event_subscribers(
  settings: Arc<Settings>,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<()> {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(build_album_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    settings.clone(),
    Arc::clone(&crawler_interactor),
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
    Arc::clone(&crawler_interactor),
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
    Arc::clone(&crawler_interactor),
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

  start_event_subscribers(
    Arc::clone(&settings),
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    Arc::clone(&crawler.crawler_interactor),
  )?;

  let album_repository = Arc::new(SqliteAlbumRepository::new(Arc::clone(&sqlite_connection)));
  let album_search_index = Arc::new(RedisAlbumSearchIndex::new(Arc::clone(
    &redis_connection_pool,
  )));
  run_rpc_server(
    Arc::clone(&settings),
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    Arc::clone(&crawler.crawler_interactor),
    Arc::clone(&parser_retry_queue),
    Arc::clone(&album_repository),
    Arc::clone(&album_search_index),
  )
  .await?;

  Ok(())
}
