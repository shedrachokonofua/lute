use super::spotify_track_search_index::SpotifyTrackSearchRecord;
use crate::{
  albums::album_read_model::AlbumReadModel,
  context::ApplicationContext,
  crawler::priority_queue::{Priority, QueuePushParameters},
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
    },
  },
  files::file_metadata::file_name::ChartParameters,
  parser::parsed_file_data::ParsedFileData,
  spotify::spotify_client::SpotifyClientError,
};
use anyhow::Result;
use chrono::{Datelike, Local, TimeDelta};
use std::sync::Arc;
use tracing::{error, info, warn};

async fn crawl_similar_albums(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
) -> Result<()> {
  if let Event::ProfileAlbumAdded { file_name, .. } = event_data.payload.event {
    let album = app_context.album_repository.get(&file_name).await?;
    let file_name_string = file_name.to_string();
    let release_type = file_name_string.split('/').collect::<Vec<&str>>()[1];

    // Artists
    for artist in album.artists {
      if let Err(e) = app_context
        .crawler
        .crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: artist.file_name,
          correlation_id: Some(format!("crawl_similar_albums:{}", file_name.to_string())),
          priority: Some(Priority::Low),
          deduplication_key: None,
          metadata: None,
        })
        .await
      {
        warn!(
          error = e.to_string(),
          "Failed to enqueue artists for similar albums"
        );
      }
    }

    // Same genres, same descriptors
    if let Err(e) = app_context
      .crawler
      .crawler_interactor
      .enqueue_if_stale(QueuePushParameters {
        file_name: ChartParameters {
          release_type: release_type.to_string(),
          page_number: 1,
          years_range_start: 1900,
          years_range_end: Local::now().year() as u32,
          include_primary_genres: Some(album.primary_genres.clone()),
          include_descriptors: Some(album.descriptors.clone()),
          ..Default::default()
        }
        .try_into()?,
        correlation_id: Some(format!("crawl_similar_albums:{}", file_name.to_string())),
        priority: Some(Priority::Low),
        deduplication_key: None,
        metadata: None,
      })
      .await
    {
      warn!(
        error = e.to_string(),
        "Failed to enqueue similar albums chart"
      );
    }

    if let Some(release_date) = album.release_date {
      // Same genres, same year
      let mut primary_genres = album.primary_genres.clone();
      primary_genres.insert(0, "all".to_string());
      if let Err(e) = app_context
        .crawler
        .crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: ChartParameters {
            release_type: release_type.to_string(),
            page_number: 1,
            years_range_start: release_date.year() as u32,
            years_range_end: release_date.year() as u32,
            include_primary_genres: Some(primary_genres),
            ..Default::default()
          }
          .try_into()?,
          correlation_id: Some(format!("crawl_similar_albums:{}", file_name.to_string())),
          priority: Some(Priority::Low),
          deduplication_key: None,
          metadata: None,
        })
        .await
      {
        warn!(
          error = e.to_string(),
          "Failed to enqueue similar albums chart"
        );
      }
    }
  }
  Ok(())
}

async fn get_spotify_track_search_records(
  app_context: Arc<ApplicationContext>,
  album: AlbumReadModel,
) -> Result<Vec<SpotifyTrackSearchRecord>> {
  let spotify_album = app_context.spotify_client.find_album(&album).await?;

  let records = if let Some(spotify_album) = spotify_album {
    let track_spotify_ids = spotify_album
      .tracks
      .iter()
      .map(|track| track.spotify_id.clone())
      .collect::<Vec<String>>();
    let mut embeddings = app_context
      .spotify_client
      .get_tracks_feature_embeddings(track_spotify_ids)
      .await?;
    spotify_album
      .tracks
      .iter()
      .filter_map(|track| {
        embeddings.remove(&track.spotify_id).map(|e| {
          SpotifyTrackSearchRecord::new(
            track.clone(),
            spotify_album.clone().into(),
            album.file_name.clone(),
            e,
          )
        })
      })
      .collect::<Vec<SpotifyTrackSearchRecord>>()
  } else {
    vec![]
  };

  Ok(records)
}

pub async fn save_album_spotify_tracks(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  subscriber_interactor: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Album(parsed_album),
  } = event_data.payload.event
  {
    let album = AlbumReadModel::from_parsed_album(&file_name, parsed_album);

    match get_spotify_track_search_records(Arc::clone(&app_context), album).await {
      Ok(records) => {
        for record in records {
          app_context.spotify_track_search_index.put(record).await?;
        }
      }
      Err(e) => {
        error!(e = e.to_string(), "Failed to get spotify track records");
        if let Some(SpotifyClientError::TooManyRequests(retry_after)) = e.downcast_ref() {
          let duration = retry_after
            .and_then(|s| TimeDelta::try_seconds(s as i64 * 2))
            .unwrap_or(TimeDelta::try_hours(1).unwrap());
          info!(
            seconds = duration.num_seconds(),
            "Pausing event subscriber due to spotify rate limit"
          );
          let _ = subscriber_interactor.pause_for(duration).await?;
        }
        return Err(e);
      }
    }
  }
  Ok(())
}

pub fn build_recommendation_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![
    EventSubscriberBuilder::default()
      .id("crawl_similar_albums".to_string())
      .topic(Topic::Profile)
      .batch_size(250)
      .app_context(Arc::clone(&app_context))
      .handle(EventHandler::Single(Arc::new(
        move |(event_data, app_context, _)| Box::pin(crawl_similar_albums(event_data, app_context)),
      )))
      .build()?,
    EventSubscriberBuilder::default()
      .id("save_album_spotify_tracks".to_string())
      .topic(Topic::Parser)
      .app_context(Arc::clone(&app_context))
      .handle(EventHandler::Single(Arc::new(
        move |(event_data, app_context, subscriber_interactor)| {
          Box::pin(save_album_spotify_tracks(
            event_data,
            app_context,
            subscriber_interactor,
          ))
        },
      )))
      .build()?,
  ])
}
