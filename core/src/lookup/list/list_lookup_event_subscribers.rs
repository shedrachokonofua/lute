use super::list_lookup_repository::ListSegmentReadModel;
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

async fn update_list_segment_read_models(
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let segments = event_data
    .into_iter()
    .filter_map(|event_data| match event_data.payload.event {
      Event::FileParsed {
        file_id: _,
        file_name,
        data: ParsedFileData::ListSegment(segment),
      } => Some(ListSegmentReadModel::try_from_parsed_list_segment(
        file_name, segment,
      )),
      _ => None,
    })
    .collect::<Result<Vec<_>>>()?;

  if !segments.is_empty() {
    app_context
      .lookup_interactor
      .put_many_list_segments(segments)
      .await?;
  }
  Ok(())
}

pub fn build_list_lookup_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![EventSubscriberBuilder::default()
    .id("update_list_segment_read_models")
    .topic(Topic::Parser)
    .batch_size(250)
    .app_context(Arc::clone(&app_context))
    .grouping_strategy(GroupingStrategy::All)
    .handler(group_event_handler!(update_list_segment_read_models))
    .build()?])
}
