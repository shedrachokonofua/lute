use super::{
  failed_parse_files_repository::{FailedParseFile, FailedParseFilesRepository},
  parser::parse_file_on_store,
};
use crate::{
  context::ApplicationContext,
  event_handler,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
    },
  },
  files::file_content_store::FileContentStore,
};
use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;

async fn parse_saved_file(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileSaved { file_id, file_name } = event_data.payload.event {
    let file_content_store = FileContentStore::new(&app_context.settings.file.content_store)?;
    parse_file_on_store(
      file_content_store,
      Arc::clone(&app_context.event_publisher),
      file_id,
      file_name,
      event_data.payload.correlation_id,
    )
    .await?;
  }
  Ok(())
}

async fn populate_failed_parse_files_repository(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let failed_parse_files_repository = FailedParseFilesRepository {
    redis_connection_pool: Arc::clone(&app_context.redis_connection_pool),
  };
  match event_data.payload.event {
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
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![
    EventSubscriberBuilder::default()
      .id("parse_saved_file")
      .app_context(Arc::clone(&app_context))
      .batch_size(app_context.settings.parser.concurrency as usize)
      .topic(Topic::File)
      .handler(event_handler!(parse_saved_file))
      .build()?,
    EventSubscriberBuilder::default()
      .id("populate_failed_parse_files_repository")
      .app_context(Arc::clone(&app_context))
      .topic(Topic::Parser)
      .handler(event_handler!(populate_failed_parse_files_repository))
      .build()?,
  ])
}
