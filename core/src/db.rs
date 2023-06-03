use crate::settings::RedisSettings;
use std::time::Duration;

pub fn build_redis_connection_pool(redis_settings: RedisSettings) -> r2d2::Pool<redis::Client> {
  let client = redis::Client::open(redis_settings.url)
    .unwrap_or_else(|e| panic!("Error connecting to redis: {}", e));

  r2d2::Pool::builder()
    .max_size(redis_settings.max_pool_size)
    .connection_timeout(Duration::from_secs(120))
    .build(client)
    .unwrap_or_else(|e| panic!("Error building redis pool: {}", e))
}
