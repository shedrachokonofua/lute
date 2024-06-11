use super::album_read_model::{
  AlbumReadModel, AlbumReadModelArtist, AlbumReadModelCredit, AlbumReadModelTrack,
};
use crate::{
  context::ApplicationContext,
  crawler::crawler::QueuePushParameters,
  event_handler,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
      GroupingStrategy,
    },
  },
  helpers::priority::Priority,
  parser::parsed_file_data::{ParsedArtistReference, ParsedCredit, ParsedFileData, ParsedTrack},
};
use anyhow::Result;
use std::sync::Arc;

impl From<&ParsedTrack> for AlbumReadModelTrack {
  fn from(parsed_track: &ParsedTrack) -> Self {
    Self {
      name: parsed_track.name.clone(),
      duration_seconds: parsed_track.duration_seconds,
      rating: parsed_track.rating,
      position: parsed_track.position.clone(),
    }
  }
}

impl From<&ParsedArtistReference> for AlbumReadModelArtist {
  fn from(parsed_artist: &ParsedArtistReference) -> Self {
    Self {
      name: parsed_artist.name.clone(),
      file_name: parsed_artist.file_name.clone(),
    }
  }
}

impl From<&ParsedCredit> for AlbumReadModelCredit {
  fn from(parsed_credit: &ParsedCredit) -> Self {
    Self {
      artist: (&parsed_credit.artist).into(),
      roles: parsed_credit.roles.clone(),
    }
  }
}

async fn update_album_read_models(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Album(parsed_album),
  } = &event_data.payload.event
  {
    let album_read_model = AlbumReadModel::from_parsed_album(file_name, parsed_album.clone());
    app_context.album_interactor.put(album_read_model).await?;
  }
  Ok(())
}

async fn delete_album_read_models(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileDeleted { file_name, .. } = &event_data.payload.event {
    app_context.album_interactor.delete(file_name).await?;
  }
  Ok(())
}

fn get_crawl_priority(correlation_id: Option<String>) -> Priority {
  correlation_id
    .map(|cid| {
      if cid.starts_with("crawl_similar_albums:") {
        Priority::Low
      } else {
        Priority::Standard
      }
    })
    .unwrap_or(Priority::Standard)
}

async fn crawl_chart_albums(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Chart(albums),
  } = event_data.payload.event
  {
    let priority = get_crawl_priority(event_data.payload.correlation_id);
    for album in albums {
      app_context
        .crawler
        .enqueue_if_stale(QueuePushParameters {
          file_name: album.file_name,
          priority: Some(priority),
          correlation_id: Some(format!("crawl_chart_albums:{}", file_name.to_string())),
          ..Default::default()
        })
        .await?;
    }
  }
  Ok(())
}

async fn crawl_artist_albums(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Artist(parsed_artist),
  } = event_data.payload.event
  {
    let priority = get_crawl_priority(event_data.payload.correlation_id);
    for album in parsed_artist.albums {
      app_context
        .crawler
        .enqueue_if_stale(QueuePushParameters {
          file_name: album.file_name,
          priority: Some(priority),
          correlation_id: Some(format!("crawl_artist_albums:{}", file_name.to_string())),
          ..Default::default()
        })
        .await?;
    }
  }
  Ok(())
}

pub fn build_album_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![
    EventSubscriberBuilder::default()
      .id("update_album_read_models")
      .topic(Topic::Parser)
      .batch_size(250)
      .app_context(Arc::clone(&app_context))
      .grouping_strategy(GroupingStrategy::GroupByKey(Arc::new(|row| {
        match &row.payload.event {
          Event::FileParsed {
            data: ParsedFileData::Album(album),
            ..
          } => album.ascii_name(), // Ensure potential duplicates are processed sequentially
          _ => "".to_string(),
        }
      })))
      .handler(event_handler!(update_album_read_models))
      .build()?,
    EventSubscriberBuilder::default()
      .id("delete_album_read_models")
      .topic(Topic::File)
      .batch_size(250)
      .app_context(Arc::clone(&app_context))
      .grouping_strategy(GroupingStrategy::GroupByKey(Arc::new(|row| {
        match &row.payload.event {
          Event::FileDeleted { file_name, .. } => file_name.to_string(),
          _ => "".to_string(),
        }
      })))
      .handler(event_handler!(delete_album_read_models))
      .build()?,
    EventSubscriberBuilder::default()
      .id("crawl_chart_albums")
      .topic(Topic::Parser)
      .batch_size(250)
      .app_context(Arc::clone(&app_context))
      .handler(event_handler!(crawl_chart_albums))
      .build()?,
    EventSubscriberBuilder::default()
      .id("crawl_artist_albums")
      .topic(Topic::Parser)
      .batch_size(250)
      .app_context(Arc::clone(&app_context))
      .handler(event_handler!(crawl_artist_albums))
      .build()?,
  ])
}
