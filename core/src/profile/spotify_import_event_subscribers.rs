use crate::{
  context::ApplicationContext,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, GroupingStrategy,
    },
  },
  lookup::album_search_lookup::AlbumSearchLookup,
};
use anyhow::Result;
use std::sync::Arc;

use super::profile_interactor::ProfileInteractor;

pub async fn process_lookup_subscriptions(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
) -> Result<()> {
  let profile_interactor = ProfileInteractor::new(
    Arc::clone(&app_context.settings),
    Arc::clone(&app_context.redis_connection_pool),
    Arc::clone(&app_context.sqlite_connection),
    Arc::clone(&app_context.album_repository),
    Arc::clone(&app_context.spotify_client),
  );
  if let Event::LookupAlbumSearchUpdated {
    lookup:
      AlbumSearchLookup::AlbumParsed {
        query,
        parsed_album_search_result,
        ..
      },
  } = event_data.payload.event
  {
    let subscriptions = profile_interactor
      .find_spotify_import_subscriptions_by_query(&query)
      .await?;
    for subscription in subscriptions {
      profile_interactor
        .put_album_on_profile(
          &subscription.profile_id,
          &parsed_album_search_result.file_name,
          subscription.factor,
        )
        .await?;
      profile_interactor
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
    .id("profile_spotify_import".to_string())
    .topic(Topic::Lookup)
    .batch_size(250)
    .app_context(Arc::clone(&app_context))
    .grouping_strategy(GroupingStrategy::GroupByCorrelationId)
    .handler(EventHandler::Single(Arc::new(
      move |(event_data, app_context, _)| {
        Box::pin(async move { process_lookup_subscriptions(event_data, app_context).await })
      },
    )))
    .build()?])
}
