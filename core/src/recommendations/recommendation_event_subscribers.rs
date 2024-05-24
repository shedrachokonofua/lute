use super::spotify_track_search_index::SpotifyTrackSearchRecord;
use crate::{
  albums::album_read_model::AlbumReadModel,
  context::ApplicationContext,
  crawler::priority_queue::{Priority, QueuePushParameters},
  event_handler,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, EventSubscriberInteractor,
      GroupingStrategy,
    },
  },
  files::file_metadata::file_name::{ChartParameters, FileName},
  group_event_handler,
  parser::parsed_file_data::ParsedFileData,
  spotify::spotify_client::SpotifyClientError,
};
use anyhow::Result;
use chrono::{Datelike, Duration, Local, TimeDelta};
use futures::future::try_join_all;
use std::{collections::HashMap, sync::Arc};
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

async fn get_spotify_track_search_records(
  app_context: Arc<ApplicationContext>,
  parsed_albums: HashMap<FileName, AlbumReadModel>,
) -> Result<Vec<SpotifyTrackSearchRecord>> {
  let parsed_albums_len = parsed_albums.len();
  let albums_processed = app_context
    .kv
    .many_exists(
      parsed_albums
        .iter()
        .map(|(file_name, _)| format!("spotify_track_embedding_album:{}", file_name.to_string()))
        .collect(),
    )
    .await?;

  let unprocessed_albums = parsed_albums
    .into_iter()
    .filter(|(file_name, _)| {
      let is_processed = albums_processed
        .get(&format!(
          "spotify_track_embedding_album:{}",
          file_name.to_string()
        ))
        .copied()
        .unwrap_or(false);
      !is_processed
    })
    .collect::<Vec<_>>();

  info!(
    unprocessed_albums_count = unprocessed_albums.len(),
    processed_albums_count = parsed_albums_len - unprocessed_albums.len(),
    "Getting uncached embeddings for album tracks"
  );

  let spotify_albums = try_join_all(
    unprocessed_albums
      .iter()
      .map(|(_, album)| app_context.spotify_client.find_album(&album)),
  )
  .await?;
  let found_album_count = spotify_albums.iter().filter(|a| a.is_some()).count();
  info!(
    parsed_album_count = parsed_albums_len,
    found_album_count, "Got spotify albums"
  );

  let tracks = unprocessed_albums
    .iter()
    .zip(spotify_albums)
    .filter_map(|((_, album), spotify_album)| {
      spotify_album.map(|spotify_album| (album, spotify_album))
    })
    .flat_map(|(album, spotify_album)| {
      spotify_album
        .tracks
        .iter()
        .map(|track| {
          (
            track.spotify_id.clone(),
            SpotifyTrackSearchRecord::new(
              track.clone(),
              spotify_album.clone().into(),
              album.file_name.clone(),
              vec![],
            ),
          )
        })
        .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>();

  let mut track_embeddings: HashMap<String, Vec<f32>> = HashMap::new();

  for chunk in tracks.chunks(100) {
    let track_ids = chunk
      .iter()
      .map(|(id, _)| id.to_string())
      .collect::<Vec<_>>();
    let embeddings = app_context
      .spotify_client
      .get_tracks_feature_embeddings(track_ids)
      .await?;
    info!(count = embeddings.len(), "Got embeddings for tracks");
    embeddings.into_iter().for_each(|(id, embedding)| {
      track_embeddings.insert(id, embedding);
    });
  }

  let tracks = tracks
    .into_iter()
    .map(|(id, mut record)| {
      record.embedding = track_embeddings.remove(&id).unwrap_or_default();
      record
    })
    .collect::<Vec<_>>();

  app_context
    .kv
    .set_many(
      unprocessed_albums
        .iter()
        .map(|(file_name, _)| {
          (
            format!("spotify_track_embedding_album:{}", file_name.to_string()),
            1,
            Duration::try_weeks(4)
              .map(|d| d.to_std())
              .transpose()
              .unwrap(),
          )
        })
        .collect::<Vec<_>>(),
    )
    .await?;

  Ok(tracks)
}

pub async fn save_album_spotify_tracks(
  events: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
  subscriber_interactor: Arc<EventSubscriberInteractor>,
) -> Result<()> {
  let parsed_albums = events
    .into_iter()
    .filter_map(|event| {
      if let Event::FileParsed {
        file_id: _,
        file_name,
        data: ParsedFileData::Album(parsed_album),
      } = event.payload.event
      {
        let album = AlbumReadModel::from_parsed_album(&file_name, parsed_album);
        Some((file_name, album))
      } else {
        None
      }
    })
    .collect::<HashMap<FileName, AlbumReadModel>>();

  if parsed_albums.is_empty() {
    return Ok(());
  }

  match get_spotify_track_search_records(Arc::clone(&app_context), parsed_albums).await {
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
      .batch_size(50)
      .grouping_strategy(GroupingStrategy::All)
      .app_context(Arc::clone(&app_context))
      .handler(group_event_handler!(save_album_spotify_tracks))
      .build()?,
  ])
}
