use super::provider::EmbeddingProvider;
use crate::{
  albums::album_read_model::AlbumReadModel,
  context::ApplicationContext,
  embedding_provider::embedding_provider_jobs::EmbeddingGenerationJobPayload,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, GroupingStrategy,
    },
  },
  parser::parsed_file_data::ParsedFileData,
  scheduler::scheduler::JobParametersBuilder,
};
use anyhow::{Ok, Result};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tracing::{info, instrument};

#[instrument(skip_all, fields(provider = provider.name()))]
async fn schedule_album_embedding_jobs(
  provider: Arc<dyn EmbeddingProvider + Send + Sync + 'static>,
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
) -> Result<()> {
  let albums = event_data
    .into_iter()
    .filter_map(|event_data: EventData| match event_data.payload.event {
      Event::FileParsed {
        file_name,
        data: ParsedFileData::Album(parsed_album),
        ..
      } => Some(AlbumReadModel::from_parsed_album(&file_name, parsed_album)),
      _ => None,
    })
    .collect::<Vec<_>>();

  if albums.is_empty() {
    return Ok(());
  }

  info!(count = albums.len(), "Scheduling album embedding jobs");

  app_context
    .scheduler
    .put_many(
      albums
        .into_iter()
        .map(|album_read_model| {
          let payload = EmbeddingGenerationJobPayload {
            provider_name: provider.name().to_string(),
            file_name: album_read_model.file_name.clone(),
            body: album_read_model.embedding_body(),
          };
          let params = JobParametersBuilder::default()
            .id(format!(
              "generate_album_embedding:{}:{}",
              provider.name(),
              album_read_model.file_name.to_string()
            ))
            .name(provider.job_name())
            .payload(serde_json::to_vec(&payload)?)
            .build()?;
          Ok(params)
        })
        .collect::<Result<Vec<_>, _>>()?,
    )
    .await?;
  Ok(())
}

#[instrument(skip_all, fields(provider = provider.name()))]
async fn schedule_artist_embedding_jobs(
  provider: Arc<dyn EmbeddingProvider + Send + Sync + 'static>,
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
) -> Result<()> {
  let start = Instant::now();
  let album_file_names = event_data
    .into_iter()
    .filter_map(|event_data: EventData| match event_data.payload.event {
      Event::AlbumSaved { file_name } => Some(file_name),
      _ => None,
    })
    .collect::<Vec<_>>();
  info!(count = album_file_names.len(), "Found album file names");

  if album_file_names.is_empty() {
    return Ok(());
  }

  let artist_file_names = app_context
    .album_interactor
    .related_artist_file_names(album_file_names)
    .await?;
  info!(count = artist_file_names.len(), "Found artist file names");

  let overviews = app_context
    .artist_interactor
    .get_overviews(artist_file_names)
    .await?
    .into_iter()
    .filter(|(_, overview)| {
      overview.album_summary.album_count >= 1 || overview.credited_album_summary.album_count >= 5
    })
    .collect::<HashMap<_, _>>();

  if overviews.is_empty() {
    return Ok(());
  }

  info!(count = overviews.len(), "Scheduling artist embedding jobs");

  app_context
    .scheduler
    .put_many(
      overviews
        .into_iter()
        .map(|(file_name, overview)| {
          let payload = EmbeddingGenerationJobPayload {
            provider_name: provider.name().to_string(),
            file_name,
            body: overview.embedding_body(),
          };
          let params = JobParametersBuilder::default()
            .id(format!(
              "generate_artist_embedding:{}:{}",
              provider.name(),
              overview.file_name.to_string()
            ))
            .name(provider.job_name())
            .payload(serde_json::to_vec(&payload)?)
            .build()?;
          Ok(params)
        })
        .collect::<Result<Vec<_>, _>>()?,
    )
    .await?;
  info!(
    "Scheduling artist embedding jobs took {:?}",
    start.elapsed()
  );
  Ok(())
}

pub fn build_embedding_provider_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  let subscribers = app_context
    .embedding_provider_interactor
    .providers
    .iter()
    .flat_map(|(_, provider)| {
      let provider_for_albums = Arc::clone(provider);
      let provider_for_artists = Arc::clone(provider);
      vec![
        EventSubscriberBuilder::default()
          .id(format!(
            "schedule_album_embedding_jobs:{}",
            provider_for_albums.name()
          ))
          .topic(Topic::Parser)
          .batch_size(250)
          .app_context(Arc::clone(&app_context))
          .grouping_strategy(GroupingStrategy::All)
          .handler(EventHandler::Group(Arc::new(
            move |(event_data, app_context, _)| {
              let provider = Arc::clone(&provider_for_albums);
              Box::pin(async move {
                schedule_album_embedding_jobs(provider, event_data, app_context).await
              })
            },
          )))
          .build(),
        EventSubscriberBuilder::default()
          .id(format!(
            "schedule_artist_embedding_jobs:{}",
            provider_for_artists.name()
          ))
          .topic(Topic::Album)
          .batch_size(50)
          .app_context(Arc::clone(&app_context))
          .grouping_strategy(GroupingStrategy::All)
          .handler(EventHandler::Group(Arc::new(
            move |(event_data, app_context, _)| {
              let provider = Arc::clone(&provider_for_artists);
              Box::pin(async move {
                schedule_artist_embedding_jobs(provider, event_data, app_context).await
              })
            },
          )))
          .build(),
      ]
    })
    .collect::<Result<Vec<EventSubscriber>, _>>()?;
  Ok(subscribers)
}
