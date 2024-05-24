use crate::{
  batch_job_executor,
  context::ApplicationContext,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobProcessorBuilder},
    scheduler_repository::Job,
  },
  spotify::spotify_client::SpotifyClientError,
};
use anyhow::Result;
use chrono::TimeDelta;
use futures::executor;
use std::sync::Arc;
use tracing::{error, info};

use super::spotify_track_search_index::SpotifyTrackSearchRecord;

async fn index_spotify_tracks(jobs: Vec<Job>, app_context: Arc<ApplicationContext>) -> Result<()> {
  let track_records = jobs
    .into_iter()
    .map(|job| {
      serde_json::from_slice::<SpotifyTrackSearchRecord>(&job.payload.expect("Job missing payload"))
        .expect("failed to parse record")
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
      if let Some(SpotifyClientError::TooManyRequests(retry_after)) = e.downcast_ref() {
        let duration = retry_after
          .and_then(|s| TimeDelta::try_seconds(s as i64 * 2))
          .unwrap_or(TimeDelta::try_hours(1).unwrap());
        info!(
          seconds = duration.num_seconds(),
          "Pausing spotify track indexing job due to spotify rate limit"
        );

        if let Err(e) = executor::block_on(
          app_context
            .scheduler
            .pause_processor(&JobName::IndexSpotifyTracks, Some(duration)),
        ) {
          error!(e = e.to_string(), "Failed to pause processor");
        }
      }
    })?;

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
        .heartbeat(TimeDelta::try_seconds(15).unwrap().to_std()?)
        .build()?,
    )
    .await;
  Ok(())
}
