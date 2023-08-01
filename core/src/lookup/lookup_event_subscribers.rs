use super::album_search_lookup_event_subscribers::build_album_search_lookup_event_subscribers;
use crate::{
  crawler::crawler_interactor::CrawlerInteractor, events::event_subscriber::EventSubscriber,
  settings::Settings,
};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub fn build_lookup_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Settings,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Vec<EventSubscriber> {
  let mut subscribers = Vec::new();
  subscribers.extend(build_album_search_lookup_event_subscribers(
    Arc::clone(&redis_connection_pool),
    settings,
    Arc::clone(&crawler_interactor),
  ));
  subscribers
}
