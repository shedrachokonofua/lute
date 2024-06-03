use super::spotify_track_search_index::SpotifyTrackQueryBuilder;
use crate::{
  albums::album_read_model::AlbumReadModel,
  context::ApplicationContext,
  crawler::crawler::QueuePushParameters,
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
};
use anyhow::Result;
use chrono::{Datelike, Local};
use std::sync::Arc;
use tracing::{info, warn};

async fn crawl_similar_albums(
  event_data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::ProfileAlbumAdded { file_name, .. } = event_data.payload.event {
    let album = app_context.album_interactor.get(&file_name).await?;
    let file_name_string = file_name.to_string();
    let release_type = file_name_string.split('/').collect::<Vec<&str>>()[1];

    // Artists
    for artist in album.artists {
      if let Err(e) = app_context
        .crawler
        .enqueue_if_stale(QueuePushParameters {
          file_name: artist.file_name,
          correlation_id: Some(format!("crawl_similar_albums:{}", file_name.to_string())),
          priority: Some(Priority::Low),
          deduplication_key: None,
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

pub async fn trigger_spotify_track_indexing(
  data: EventData,
  app_context: Arc<ApplicationContext>,
  _: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Album(parsed_album),
  } = data.payload.event
  {
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

    if app_context
      .spotify_track_search_index
      .search(
        &SpotifyTrackQueryBuilder::default()
          .include_album_file_names(vec![file_name.clone()])
          .build()?,
        None,
      )
      .await?
      .total
      > 0
    {
      info!(
        "Spotify tracks for album {} already indexed, found in search index",
        file_name.to_string()
      );
      return Ok(());
    }

    let album = AlbumReadModel::from_parsed_album(&file_name, parsed_album);
    if let Some(spotify_id) = &album.spotify_id {
      app_context
        .scheduler
        .put(
          JobParametersBuilder::default()
            .id(format!("fetch_spotify_tracks_by_album_ids:{}", spotify_id))
            .name(JobName::FetchSpotifyTracksByAlbumIds)
            .payload(serde_json::to_vec(&album)?)
            .build()?,
        )
        .await?;
      app_context
        .scheduler
        .delete_job(&format!(
          "fetch_spotify_tracks_by_album_search:{}",
          file_name.to_string()
        ))
        .await?;
    } else {
      app_context
        .scheduler
        .put(
          JobParametersBuilder::default()
            .id(format!(
              "fetch_spotify_tracks_by_album_search:{}",
              file_name.to_string()
            ))
            .name(JobName::FetchSpotifyTracksByAlbumSearch)
            .payload(serde_json::to_vec(&album)?)
            .build()?,
        )
        .await?;
    };
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
      .id("trigger_spotify_track_indexing")
      .topic(Topic::Parser)
      .batch_size(250)
      .app_context(Arc::clone(&app_context))
      .handler(event_handler!(trigger_spotify_track_indexing))
      .build()?,
  ])
}
