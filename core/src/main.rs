pub mod db;
pub mod events;
pub mod files;
pub mod proto;
pub mod rpc;
pub mod settings;

use db::build_redis_connection_pool;
use dotenv::dotenv;
use events::event_subscriber::EventSubscriber;
use files::file_event_subscribers::get_file_event_subscribers;
use rpc::server::RpcServer;
use settings::Settings;
use std::sync::Arc;
use tokio::task;

fn get_rpc_server_task(
  settings: Settings,
  redis_connection_pool: Arc<r2d2::Pool<redis::Client>>,
) -> task::JoinHandle<()> {
  let rpc_server = RpcServer::new(settings.clone(), redis_connection_pool.clone());

  task::spawn(async move {
    rpc_server.run().await.unwrap();
  })
}

fn start_event_subscribers(redis_connection_pool: Arc<r2d2::Pool<redis::Client>>) {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(get_file_event_subscribers(redis_connection_pool.clone()));
  event_subscribers.into_iter().for_each(|subscriber| {
    task::spawn(async move { subscriber.run() });
  });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();
  let settings: Settings = Settings::new()?;
  let redis_connection_pool = Arc::new(build_redis_connection_pool(settings.redis.clone()));

  start_event_subscribers(redis_connection_pool.clone());
  get_rpc_server_task(settings, redis_connection_pool.clone()).await?;

  Ok(())
}
