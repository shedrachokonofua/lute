use crate::{
  context::ApplicationContext,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
      GroupingStrategy,
    },
  },
  group_event_handler,
  parser::parsed_file_data::ParsedFileData,
};
use anyhow::Result;
use std::sync::Arc;

use super::list_segment_repository::ListSegmentDocument;

async fn put_list_segment_document(
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let docs = event_data
    .into_iter()
    .filter_map(|event_data| match event_data.payload.event {
      Event::FileParsed {
        file_id: _,
        file_name,
        data: ParsedFileData::ListSegment(segment),
      } => Some(ListSegmentDocument::try_from_parsed_list_segment(
        file_name, segment,
      )),
      _ => None,
    })
    .collect::<Result<Vec<_>>>()?;

  if !docs.is_empty() {
    app_context
      .lookup_interactor
      .put_many_list_segment(docs)
      .await?;
  }
  Ok(())
}

pub fn build_list_lookup_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![EventSubscriberBuilder::default()
    .id("put_list_segment_document")
    .topic(Topic::Parser)
    .batch_size(250)
    .app_context(Arc::clone(&app_context))
    .grouping_strategy(GroupingStrategy::All)
    .handler(group_event_handler!(put_list_segment_document))
    .build()?])
}
