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
  if let Event::FileSaved { file_id, file_name } = context.payload.event {
    let file_content_store = FileContentStore::new(context.settings.file.content_store.clone())?;
    let event_publisher = EventPublisher::new(Arc::clone(&context.redis_connection_pool));
    parse_file_on_store(file_content_store, event_publisher, file_id, file_name).await?;
  }
  Ok(())
}

pub fn build_parser_event_subscribers(
  redis_connection_pool: Arc<Pool<Client>>,
  settings: Settings,
) -> Vec<EventSubscriber> {
  vec![EventSubscriber {
    redis_connection_pool,
    settings,
    id: "parse_saved_file".to_string(),
    concurrency: Some(50),
    stream: Stream::File,
    handle: Arc::new(|context| Box::pin(async move { parse_saved_file(context).await })),
  }]
}
