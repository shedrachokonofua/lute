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
};
use anyhow::Result;
use std::{collections::HashSet, sync::Arc};
use tracing::info;

pub async fn update_artist_search_records(
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let album_file_names = event_data
    .into_iter()
    .filter_map(|event_data: EventData| match event_data.payload.event {
      Event::AlbumSaved { file_name } => Some(file_name),
      _ => None,
    })
    .collect::<Vec<_>>();
  info!(count = album_file_names.len(), "Found album file names");

  let albums = app_context
    .album_interactor
    .find_many(album_file_names)
    .await?;
  info!(count = albums.len(), "Found albums");

  let artist_file_names = albums
    .values()
    .flat_map(|album| {
      album
        .credits
        .iter()
        .map(|credit| credit.artist.file_name.clone())
        .chain(album.artists.iter().map(|artist| artist.file_name.clone()))
    })
    .collect::<HashSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
  info!(count = artist_file_names.len(), "Found artist file names");

  app_context
    .artist_interactor
    .update_search_records(artist_file_names)
    .await?;
  Ok(())
}

pub fn build_artist_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![EventSubscriberBuilder::default()
    .id("update_artist_search_records")
    .topic(Topic::Album)
    .batch_size(75)
    .app_context(Arc::clone(&app_context))
    .grouping_strategy(GroupingStrategy::All)
    .handler(group_event_handler!(update_artist_search_records))
    .build()?])
}
