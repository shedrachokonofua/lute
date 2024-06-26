use crate::{
  albums::redis_album_search_index::RedisAlbumSearchIndex, context::ApplicationContext,
  recommendations::spotify_track_search_index::SpotifyTrackSearchIndex, settings::RedisSettings,
};
use anyhow::Result;
use rustis::{
  bb8::{ErrorSink, Pool},
  client::PooledClientManager,
};
use std::{sync::Arc, time::Duration};
use tracing::error;

#[derive(Debug)]
struct RedisConnectionErrorSink;

impl ErrorSink<rustis::Error> for RedisConnectionErrorSink {
  fn sink(&self, error: rustis::Error) {
    error!("Redis connection error: {:?}", error);
  }

  fn boxed_clone(&self) -> Box<dyn ErrorSink<rustis::Error>> {
    Box::new(RedisConnectionErrorSink {})
  }
}

pub async fn build_redis_connection_pool(
  redis_settings: RedisSettings,
) -> Result<Pool<PooledClientManager>> {
  let error_sink = RedisConnectionErrorSink {};
  Pool::builder()
    .min_idle(Some(1))
    .max_size(redis_settings.max_pool_size)
    .connection_timeout(Duration::from_secs(30))
    .error_sink(Box::new(error_sink))
    .build(PooledClientManager::new(redis_settings.url.as_str())?)
    .await
    .map_err(|e| e.into())
}

pub async fn setup_redis_indexes(app_context: Arc<ApplicationContext>) -> Result<()> {
  RedisAlbumSearchIndex::new(
    Arc::clone(&app_context.redis_connection_pool),
    Arc::clone(&app_context.embedding_provider_interactor),
  )
  .setup_index()
  .await?;

  SpotifyTrackSearchIndex::new(Arc::clone(&app_context.redis_connection_pool))
    .setup_index()
    .await?;

  Ok(())
}
