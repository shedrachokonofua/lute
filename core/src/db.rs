use crate::{
  parser::failed_parse_files_repository::FailedParseFilesRepository, settings::RedisSettings,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::{sync::Arc, time::Duration};

pub async fn build_redis_connection_pool(
  redis_settings: RedisSettings,
) -> Result<Pool<PooledClientManager>> {
  Pool::builder()
    .min_idle(Some(1))
    .max_size(redis_settings.max_pool_size)
    .connection_timeout(Duration::from_secs(120))
    .build(PooledClientManager::new(redis_settings.url.as_str())?)
    .await
    .map_err(|e| e.into())
}

pub async fn setup_redis_indexes(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
) -> Result<()> {
  FailedParseFilesRepository {
    redis_connection_pool: redis_connection_pool.clone(),
  }
  .setup_index()
  .await?;

  Ok(())
}
