use super::spotify_import_event_subscribers::build_spotify_import_event_subscribers;
use crate::{context::ApplicationContext, events::event_subscriber::EventSubscriber};
use anyhow::Result;
use std::sync::Arc;

pub fn build_profile_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  let mut subscribers = vec![];
  subscribers.extend(build_spotify_import_event_subscribers(app_context)?);
  Ok(subscribers)
}
