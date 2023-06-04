use super::parser::parse_file_on_store;
use crate::{
  events::{
    event::{Event, Stream},
    event_publisher::EventPublisher,
    event_subscriber::{EventSubscriber, SubscriberContext},
  },
  files::file_content_store::FileContentStore,
  settings::Settings,
};
use anyhow::Result;
use r2d2::Pool;
use redis::Client;
use std::sync::Arc;

async fn parse_saved_file(context: SubscriberContext) -> Result<()> {
  match context.payload.event {
    Event::FileSaved { file_id, file_name } => {
      let file_content_store = FileContentStore::new(context.settings.file.content_store.clone())?;
      let event_publisher = EventPublisher::new(context.redis_connection_pool.clone());
      parse_file_on_store(file_content_store, event_publisher, file_id, file_name).await?;
    }
    _ => (),
  }
  Ok(())
}

pub fn get_parser_event_subscribers(
  redis_connection_pool: Arc<Pool<Client>>,
  settings: Settings,
) -> Vec<EventSubscriber> {
  vec![EventSubscriber {
    redis_connection_pool: redis_connection_pool.clone(),
    settings: settings.clone(),
    id: "parse_saved_file".to_string(),
    stream: Stream::File,
    handle: Arc::new(|content| Box::pin(async move { parse_saved_file(content).await })),
  }]
}
