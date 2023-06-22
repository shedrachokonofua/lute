use core::{
  crawler::crawler::Crawler, db::build_redis_connection_pool,
  events::event_subscriber::EventSubscriber, log::setup_logging,
  parser::parser_event_subscribers::get_parser_event_subscribers, rpc::RpcServer,
  settings::Settings,
};
use dotenv::dotenv;
use std::sync::Arc;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::task;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn run_rpc_server(
  settings: Settings,
  redis_connection_pool: Arc<r2d2::Pool<redis::Client>>,
  crawler: Arc<Crawler>,
) -> task::JoinHandle<()> {
  let rpc_server = RpcServer::new(settings, redis_connection_pool, crawler);

  task::spawn(async move {
    rpc_server.run().await.unwrap();
  })
}

fn start_event_subscribers(
  settings: Settings,
  redis_connection_pool: Arc<r2d2::Pool<redis::Client>>,
) {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(get_parser_event_subscribers(
    redis_connection_pool,
    settings,
  ));

  event_subscribers.into_iter().for_each(|subscriber| {
    task::spawn(async move { subscriber.run().await });
  });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();
  setup_logging();

  let settings: Settings = Settings::new()?;
  let redis_connection_pool = Arc::new(build_redis_connection_pool(settings.redis.clone()));
  let crawler = Arc::new(Crawler::new(
    settings.clone(),
    redis_connection_pool.clone(),
  ));

  crawler.run()?;
  start_event_subscribers(settings.clone(), redis_connection_pool.clone());
  run_rpc_server(settings, redis_connection_pool.clone(), crawler.clone()).await?;

  Ok(())
}
