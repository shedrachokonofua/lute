use crate::{
  albums::album_repository::AlbumRepository,
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, EventSubscriberBuilder, SubscriberContext},
  },
  lookup::album_search_lookup::AlbumSearchLookup,
  settings::Settings,
  sqlite::SqliteConnection,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

use super::profile_interactor::ProfileInteractor;

pub async fn process_lookup_subscriptions(
  context: SubscriberContext,
  profile_interactor: ProfileInteractor,
) -> Result<()> {
  if let Event::LookupAlbumSearchUpdated {
    lookup:
      AlbumSearchLookup::AlbumParsed {
        query,
        parsed_album_search_result,
        ..
      },
  } = context.payload.event
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
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<SqliteConnection>,
  settings: Arc<Settings>,
  album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
) -> Result<Vec<EventSubscriber>> {
  let album_repository = Arc::clone(&album_repository);
  Ok(vec![EventSubscriberBuilder::default()
    .id("profile_spotify_import".to_string())
    .stream(Stream::Lookup)
    .batch_size(250)
    .redis_connection_pool(Arc::clone(&redis_connection_pool))
    .sqlite_connection(Arc::clone(&sqlite_connection))
    .settings(Arc::clone(&settings))
    .generate_ordered_processing_group_id(Arc::new(|row| {
      if let Some(correlation_id) = &row.payload.correlation_id {
        Some(correlation_id.clone())
      } else {
        None
      }
    }))
    .handle(Arc::new(move |context| {
      let profile_interactor = ProfileInteractor::new(
        Arc::clone(&context.settings),
        Arc::clone(&context.redis_connection_pool),
        Arc::clone(&context.sqlite_connection),
        Arc::clone(&album_repository),
      );
      Box::pin(async move { process_lookup_subscriptions(context, profile_interactor).await })
    }))
    .build()?])
}
