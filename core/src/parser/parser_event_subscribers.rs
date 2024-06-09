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
      GroupingStrategy,
    },
  },
  group_event_handler,
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
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let parser_failure_repository = ParserFailureRepository::new(Arc::clone(&app_context.doc_store));

  let (failures, parsed_file_names) = event_data.into_iter().fold(
    (Vec::new(), Vec::new()),
    |(mut failures, mut parsed_file_names), event_data| {
      match event_data.payload.event {
        Event::FileParseFailed {
          file_id: _,
          file_name,
          error,
        } => {
          failures.push(ParserFailure {
            file_name,
            error,
            last_attempted_at: Utc::now().naive_utc(),
          });
        }
        Event::FileParsed {
          file_id: _,
          file_name,
          data: _,
        } => {
          parsed_file_names.push(file_name);
        }
        _ => {}
      }
      (failures, parsed_file_names)
    },
  );

  if !failures.is_empty() {
    parser_failure_repository.put_many(failures).await?;
  }

  if !parsed_file_names.is_empty() {
    parser_failure_repository
      .delete_many(parsed_file_names)
      .await?;
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
      .grouping_strategy(GroupingStrategy::All)
      .handler(group_event_handler!(populate_parser_failure_repository))
      .build()?,
  ])
}
