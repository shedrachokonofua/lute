use super::spotify_track_search_index::SpotifyTrackSearchRecord;
use crate::{
  albums::album_read_model::AlbumReadModel,
  context::ApplicationContext,
  crawler::priority_queue::QueuePushParameters,
  event_handler,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
    },
  },
  files::file_metadata::file_name::ChartParameters,
  helpers::priority::Priority,
  parser::parsed_file_data::ParsedFileData,
  scheduler::{job_name::JobName, scheduler::JobParametersBuilder},
  spotify::spotify_client::get_spotify_retry_after,
};
use anyhow::Result;
use chrono::{Datelike, Duration, Local};
use std::sync::Arc;
use tokio::spawn;
use tracing::{error, info, warn};

async fn crawl_similar_albums(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
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

pub async fn save_album_spotify_tracks(
  data: EventData,
  app_context: Arc<ApplicationContext>,
  subscriber_interactor: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Album(parsed_album),
  } = data.payload.event
  {
    let album = AlbumReadModel::from_parsed_album(&file_name, parsed_album);

    if app_context
      .kv
      .exists(format!(
        "spotify_track_embedding_album:{}",
        file_name.to_string()
      ))
      .await?
    {
      info!(
        "Spotify tracks for album {} already indexed",
        file_name.to_string()
      );
      return Ok(());
    }

    let spotify_album = app_context
      .spotify_client
      .find_album(&album)
      .await
      .inspect_err(|e| {
        error!(e = e.to_string(), "Failed to get spotify track records");
        if let Some(retry_after) = get_spotify_retry_after(e) {
          info!(
            seconds = retry_after.num_seconds(),
            "Pausing spotify track indexing job due to spotify rate limit"
          );

          spawn(async move {
            if let Err(e) = subscriber_interactor.pause_for(retry_after).await {
              error!(e = e.to_string(), "Failed to pause processor");
            }
          });
        }
      })?;

    if let Some(spotify_album) = spotify_album {
      let tracks = spotify_album
        .tracks
        .iter()
        .map(|track| {
          (
            track.spotify_id.clone(),
            SpotifyTrackSearchRecord::new(
              track.clone(),
              spotify_album.clone().into(),
              file_name.clone(),
              vec![],
            ),
          )
        })
        .collect::<Vec<_>>();

      for (spotify_id, record) in tracks {
        app_context
          .scheduler
          .put(
            JobParametersBuilder::default()
              .id(format!("save_album_spotify_tracks:{}", spotify_id))
              .name(JobName::IndexSpotifyTracks)
              .payload(Some(serde_json::to_vec(&record)?))
              .build()?,
          )
          .await?;
      }

      app_context
        .kv
        .set(
          &format!("spotify_track_embedding_album:{}", file_name.to_string()),
          1,
          Duration::try_weeks(4)
            .map(|d| d.to_std())
            .transpose()
            .unwrap(),
        )
        .await?;
    }
  }

  Ok(())
}

pub fn build_recommendation_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![
    EventSubscriberBuilder::default()
      .id("crawl_similar_albums")
      .topic(Topic::Profile)
      .batch_size(250)
      .app_context(Arc::clone(&app_context))
      .handler(event_handler!(crawl_similar_albums))
      .build()?,
    EventSubscriberBuilder::default()
      .id("save_album_spotify_tracks")
      .topic(Topic::Parser)
      .batch_size(2)
      .app_context(Arc::clone(&app_context))
      .handler(event_handler!(save_album_spotify_tracks))
      .build()?,
  ])
}
