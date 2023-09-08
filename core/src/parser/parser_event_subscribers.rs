use super::{
  failed_parse_files_repository::{FailedParseFile, FailedParseFilesRepository},
  parser::parse_file_on_store,
};
use crate::{
  events::{
    event::{Event, Stream},
    event_publisher::EventPublisher,
    event_subscriber::{EventSubscriber, EventSubscriberBuilder, SubscriberContext},
  },
  files::file_content_store::FileContentStore,
  settings::Settings,
};
use anyhow::Result;
use chrono::Utc;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

async fn parse_saved_file(context: SubscriberContext) -> Result<()> {
  if let Event::FileSaved { file_id, file_name } = context.payload.event {
    let file_content_store = FileContentStore::new(&context.settings.file.content_store)?;
    let event_publisher = EventPublisher::new(
      Arc::clone(&context.settings),
      Arc::clone(&context.sqlite_connection),
    );
    parse_file_on_store(
      file_content_store,
      event_publisher,
      file_id,
      file_name,
      context.payload.correlation_id,
    )
    .await?;
  }
  Ok(())
}

async fn populate_failed_parse_files_repository(context: SubscriberContext) -> Result<()> {
  let failed_parse_files_repository = FailedParseFilesRepository {
    redis_connection_pool: Arc::clone(&context.redis_connection_pool),
  };
  match context.payload.event {
    Event::FileParseFailed {
      file_id: _,
      file_name,
      error,
    } => {
      failed_parse_files_repository
        .put(FailedParseFile {
          file_name,
          error,
          last_attempted_at: Utc::now().naive_utc(),
        })
        .await?;
    }
    Event::FileParsed {
      file_id: _,
      file_name,
      data: _,
    } => {
      failed_parse_files_repository.remove(&file_name).await?;
    }
    _ => {}
  }
  Ok(())
}

pub fn build_parser_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
  settings: Arc<Settings>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![
    EventSubscriberBuilder::default()
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .sqlite_connection(Arc::clone(&sqlite_connection))
      .settings(Arc::clone(&settings))
      .id("parse_saved_file".to_string())
      .batch_size(settings.parser.concurrency as usize)
      .stream(Stream::File)
      .handle(Arc::new(|context| {
        Box::pin(async move { parse_saved_file(context).await })
      }))
      .build()?,
    EventSubscriberBuilder::default()
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .sqlite_connection(Arc::clone(&sqlite_connection))
      .settings(Arc::clone(&settings))
      .id("populate_failed_parse_files_repository".to_string())
      .batch_size(1)
      .stream(Stream::Parser)
      .handle(Arc::new(|context| {
        Box::pin(async move { populate_failed_parse_files_repository(context).await })
      }))
      .build()?,
  ])
}
