use crate::{
  batch_job_executor,
  context::ApplicationContext,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobProcessorBuilder},
    scheduler_repository::Job,
  },
  spotify::spotify_client::get_spotify_retry_after,
};
use anyhow::Result;
use chrono::TimeDelta;
use std::sync::Arc;
use tokio::spawn;
use tracing::{error, info};

use super::spotify_track_search_index::SpotifyTrackSearchRecord;

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

pub async fn setup_recommendation_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
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
