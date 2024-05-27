use super::spotify_track_search_index::SpotifyTrackSearchRecord;
use crate::{
  albums::album_read_model::AlbumReadModel,
  batch_job_executor,
  context::ApplicationContext,
  files::file_metadata::file_name::FileName,
  job_executor,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobParametersBuilder, JobProcessorBuilder},
    scheduler_repository::Job,
  },
  spotify::spotify_client::{get_spotify_retry_after, SpotifyAlbum},
};
use anyhow::{anyhow, Result};
use chrono::{Duration, TimeDelta};
use std::{collections::HashMap, sync::Arc};
use tokio::spawn;
use tracing::{error, info, warn};

async fn index_spotify_tracks(jobs: Vec<Job>, app_context: Arc<ApplicationContext>) -> Result<()> {
  let track_records = jobs
    .into_iter()
    .filter_map(|job| {
      job
        .payload::<SpotifyTrackSearchRecord>()
        .inspect_err(|e| {
          error!(
            e = e.to_string(),
            "Failed to get spotify track search record"
          );
        })
        .ok()
    })
    .collect::<Vec<_>>();

  let track_ids = track_records
    .iter()
    .map(|r| r.spotify_id.clone())
    .collect::<Vec<_>>();

  let embeddings = app_context
    .spotify_client
    .get_tracks_feature_embeddings(track_ids)
    .await
    .inspect_err(|e| {
      error!(e = e.to_string(), "Failed to get spotify track records");
      if let Some(retry_after) = get_spotify_retry_after(e) {
        info!(
          seconds = retry_after.num_seconds(),
          "Pausing spotify track indexing job due to spotify rate limit"
        );
        let app_context = Arc::clone(&app_context);
        spawn(async move {
          if let Err(e) = app_context
            .scheduler
            .pause_processor(&JobName::IndexSpotifyTracks, Some(retry_after))
            .await
          {
            error!(e = e.to_string(), "Failed to pause processor");
          }
        });
      }
    })?;

  info!("Got embeddings for {} tracks", embeddings.len());

  let records = track_records
    .into_iter()
    .zip(embeddings)
    .map(|(record, (_, embedding))| SpotifyTrackSearchRecord {
      embedding,
      ..record
    })
    .collect::<Vec<_>>();

  for record in records {
    app_context.spotify_track_search_index.put(record).await?;
  }

  Ok(())
}

pub async fn schedule_track_indexing(
  app_context: Arc<ApplicationContext>,
  file_name: FileName,
  album: SpotifyAlbum,
) -> Result<()> {
  let tracks = album
    .tracks
    .iter()
    .map(|track| {
      (
        track.spotify_id.clone(),
        SpotifyTrackSearchRecord::new(
          track.clone(),
          album.clone().into(),
          file_name.clone(),
          vec![],
        ),
      )
    })
    .collect::<Vec<_>>();

  info!(
    album_id = album.spotify_id,
    album_name = album.name,
    tracks = tracks.len(),
    "Scheduling spotify track indexing job"
  );

  for (spotify_id, record) in tracks {
    app_context
      .scheduler
      .put(
        JobParametersBuilder::default()
          .id(format!("save_album_spotify_tracks:{}", spotify_id))
          .name(JobName::IndexSpotifyTracks)
          .payload(serde_json::to_vec(&record)?)
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
  Ok(())
}

pub async fn fetch_spotify_tracks_by_album_ids(
  jobs: Vec<Job>,
  app_context: Arc<ApplicationContext>,
) -> Result<()> {
  let albums = jobs
    .into_iter()
    .map(|job| {
      job.payload::<AlbumReadModel>().and_then(|a| {
        if !a.spotify_id.is_none() {
          Ok(a)
        } else {
          Err(anyhow!("No spotify id"))
        }
      })
    })
    .collect::<Result<Vec<_>>>()?;

  let albums_by_spotify_id = albums
    .into_iter()
    .map(|a| (a.spotify_id.clone().unwrap(), a))
    .collect::<HashMap<_, _>>();

  let album_pages = app_context
    .spotify_client
    .get_album_pages(albums_by_spotify_id.keys().cloned().collect::<Vec<_>>())
    .await
    .inspect_err(|e| {
      error!(e = e.to_string(), "Failed to get spotify album pages");
      if let Some(retry_after) = get_spotify_retry_after(e) {
        info!(
          seconds = retry_after.num_seconds(),
          "Pausing spotify track indexing job due to spotify rate limit"
        );
        let app_context = Arc::clone(&app_context);
        spawn(async move {
          if let Err(e) = app_context
            .scheduler
            .pause_processor(&JobName::FetchSpotifyTracksByAlbumIds, Some(retry_after))
            .await
          {
            error!(e = e.to_string(), "Failed to pause processor");
          }
        });
      }
    })?;

  let mut incomplete_albums = 0;
  for page in album_pages.iter() {
    if page.has_more_tracks {
      warn!(
        id = page.spotify_album.spotify_id,
        name = page.spotify_album.name,
        "Album has more tracks than can be fetched in a single request",
      );
      incomplete_albums += 1;
    }
  }

  info!(
    albums = album_pages.len(),
    incomplete_albums = incomplete_albums,
    "Got spotify album pages"
  );

  for page in album_pages {
    let album = albums_by_spotify_id
      .get(&page.spotify_album.spotify_id.replace("spotify:album:", ""))
      .ok_or_else(|| anyhow!("Album not found"))?;
    schedule_track_indexing(
      Arc::clone(&app_context),
      album.file_name.clone(),
      page.spotify_album.clone(),
    )
    .await?;
  }

  Ok(())
}

pub async fn fetch_spotify_tracks_by_album_search(
  job: Job,
  app_context: Arc<ApplicationContext>,
) -> Result<()> {
  let album = job.payload::<AlbumReadModel>()?;

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
        let app_context = Arc::clone(&app_context);
        spawn(async move {
          if let Err(e) = app_context
            .scheduler
            .pause_processor(&JobName::FetchSpotifyTracksByAlbumSearch, Some(retry_after))
            .await
          {
            error!(e = e.to_string(), "Failed to pause processor");
          }
        });
      }
    })?;

  if let Some(spotify_album) = spotify_album {
    schedule_track_indexing(Arc::clone(&app_context), album.file_name, spotify_album).await?;
  }

  Ok(())
}

pub async fn setup_recommendation_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::FetchSpotifyTracksByAlbumIds)
        .app_context(Arc::clone(&app_context))
        .concurrency(2)
        .executor(batch_job_executor!(fetch_spotify_tracks_by_album_ids, 20))
        .build()?,
    )
    .await;

  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::FetchSpotifyTracksByAlbumSearch)
        .app_context(Arc::clone(&app_context))
        .concurrency(2)
        .executor(job_executor!(fetch_spotify_tracks_by_album_search))
        .cooldown(TimeDelta::try_seconds(10).unwrap().to_std()?)
        .build()?,
    )
    .await;

  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::IndexSpotifyTracks)
        .app_context(Arc::clone(&app_context))
        .executor(batch_job_executor!(index_spotify_tracks, 100))
        .cooldown(TimeDelta::try_seconds(15).unwrap().to_std()?)
        .build()?,
    )
    .await;
  Ok(())
}
