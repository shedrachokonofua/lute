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
  lookup::AlbumSearchLookup,
};
use anyhow::Result;
use std::sync::Arc;

pub async fn process_lookup_subscriptions(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::LookupAlbumSearchUpdated {
    lookup:
      AlbumSearchLookup::AlbumParsed {
        query,
        parsed_album_search_result,
        ..
      },
  } = event_data.payload.event
  {
    let subscriptions = app_context
      .profile_interactor
      .find_spotify_import_subscriptions_by_query(&query)
      .await?;
    for subscription in subscriptions {
      app_context
        .profile_interactor
        .put_album_on_profile(
          &subscription.profile_id,
          &parsed_album_search_result.file_name,
          subscription.factor,
        )
        .await?;
      app_context
        .profile_interactor
        .delete_spotify_import_subscription(&subscription.profile_id, &query)
        .await?;
    }
  }
  Ok(())
}

pub fn build_spotify_import_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![EventSubscriberBuilder::default()
    .id("profile_spotify_import")
    .topic(Topic::Lookup)
    .batch_size(250)
    .app_context(Arc::clone(&app_context))
    .grouping_strategy(GroupingStrategy::GroupByCorrelationId)
    .handler(event_handler!(process_lookup_subscriptions))
    .build()?])
}
