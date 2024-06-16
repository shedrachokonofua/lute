use super::album_search::build_album_search_lookup_event_subscribers;
use crate::{context::ApplicationContext, events::event_subscriber::EventSubscriber};
use anyhow::Result;
use std::sync::Arc;

pub fn build_lookup_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  let mut subscribers = Vec::new();
  subscribers.extend(build_album_search_lookup_event_subscribers(app_context)?);
  Ok(subscribers)
}
