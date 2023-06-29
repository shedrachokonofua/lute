use crate::{
  parser::failed_parse_files_repository::FailedParseFilesRepository, settings::RedisSettings,
};
use anyhow::Result;
use r2d2::Pool;
use redis::Client;
use std::{sync::Arc, time::Duration};

pub fn build_redis_connection_pool(redis_settings: RedisSettings) -> Pool<Client> {
  let client =
    Client::open(redis_settings.url).unwrap_or_else(|e| panic!("Error connecting to redis: {}", e));

  Pool::builder()
    .min_idle(Some(1))
    .max_size(redis_settings.max_pool_size)
    .connection_timeout(Duration::from_secs(120))
    .build(client)
    .unwrap_or_else(|e| panic!("Error building redis pool: {}", e))
}

pub fn setup_redis_indexes(redis_connection_pool: Arc<Pool<Client>>) -> Result<()> {
  FailedParseFilesRepository {
    redis_connection_pool: redis_connection_pool.clone(),
  }
  .setup_index()?;

  Ok(())
}
