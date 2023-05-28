use core::{db::build_redis_connection_pool, rpc::server::RpcServer, settings::Settings};
use dotenv::dotenv;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenv().ok();
  let settings: Settings = Settings::new()?;
  let redis_connection_pool = Arc::new(build_redis_connection_pool(settings.redis.clone()));
  let rpc_server = RpcServer::new(settings.clone(), redis_connection_pool.clone());
  rpc_server.run().await?;

  Ok(())
}
