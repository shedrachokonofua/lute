use super::album_search_lookup_event_subscribers::build_album_search_lookup_event_subscribers;
use crate::{
  crawler::crawler_interactor::CrawlerInteractor, events::event_subscriber::EventSubscriber,
  settings::Settings, sqlite::SqliteConnection,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub fn build_lookup_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<SqliteConnection>,
  settings: Arc<Settings>,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<Vec<EventSubscriber>> {
  let mut subscribers = Vec::new();
  subscribers.extend(build_album_search_lookup_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    settings,
    Arc::clone(&crawler_interactor),
  )?);
  Ok(subscribers)
}
