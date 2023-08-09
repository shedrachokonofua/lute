use super::spotify_import_event_subscribers::build_spotify_import_event_subscribers;
use crate::{events::event_subscriber::EventSubscriber, settings::Settings};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub fn build_profile_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Arc<Settings>,
) -> Vec<EventSubscriber> {
  let mut subscribers = vec![];
  subscribers.extend(build_spotify_import_event_subscribers(
    Arc::clone(&redis_connection_pool),
    settings,
  ));
  subscribers
}
