use super::{
  parser::parse_file_on_store,
  parser_failure_repository::{ParserFailure, ParserFailureRepository},
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
    parse_file_on_store(
      app_context,
      file_id,
      file_name,
      event_data.payload.correlation_id,
    )
    .await?;
  }
  Ok(())
}

async fn populate_parser_failure_repository(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let parser_failure_repository = ParserFailureRepository::new(Arc::clone(&app_context.doc_store));

  match event_data.payload.event {
    Event::FileParseFailed {
      file_id: _,
      file_name,
      error,
    } => {
      parser_failure_repository
        .put(ParserFailure {
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
      parser_failure_repository.remove(&file_name).await?;
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
      .id("populate_parser_failure_repository")
      .app_context(Arc::clone(&app_context))
      .topic(Topic::Parser)
      .batch_size(250)
      .handler(event_handler!(populate_parser_failure_repository))
      .build()?,
  ])
}
