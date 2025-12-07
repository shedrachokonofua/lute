use crate::{
  context::ApplicationContext,
  embedding_provider::embedding_provider_jobs::EmbeddingGenerationJobPayload,
  events::{
    event::{Event, Topic},
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, GroupingStrategy,
    },
  },
  scheduler::scheduler::JobParametersBuilder,
};
use anyhow::Result;
use std::sync::Arc;
use tracing::{info, instrument, warn};

const OPENAI_KEY: &str = "openai-default";

// Hardcoded keys to delete - includes removed providers and current provider for full regeneration
const KEYS_TO_DELETE: &[&str] = &["openai-default", "voyageai-default"];

#[instrument(skip_all, fields(count = event_data.len()))]
async fn handle_album_saved_for_embedding_migration(
  event_data: Vec<EventData>,
  app_context: Arc<ApplicationContext>,
) -> Result<()> {
  let file_names = event_data
    .into_iter()
    .filter_map(|event_data| match event_data.payload.event {
      Event::AlbumSaved { file_name } => Some(file_name),
      _ => None,
    })
    .collect::<Vec<_>>();

  if file_names.is_empty() {
    return Ok(());
  }

  info!(
    count = file_names.len(),
    "Processing album embedding migration"
  );

  let openai_provider = app_context
    .embedding_provider_interactor
    .get_provider_by_name(OPENAI_KEY)
    .ok();

  for file_name in file_names {
    info!(
      file_name = file_name.to_string(),
      keys_to_delete = ?KEYS_TO_DELETE,
      "Starting embedding migration for album"
    );

    // Delete all embeddings (hardcoded keys since removed providers won't be in config)
    for key in KEYS_TO_DELETE {
      info!(
        file_name = file_name.to_string(),
        key = key,
        "Attempting to delete embedding"
      );
      match app_context
        .album_interactor
        .delete_embedding(&file_name, key)
        .await
      {
        Ok(_) => {
          info!(
            file_name = file_name.to_string(),
            key = key,
            "Successfully deleted embedding"
          );
        }
        Err(e) => {
          warn!(
            file_name = file_name.to_string(),
            key = key,
            error = e.to_string(),
            "Failed to delete embedding"
          );
        }
      }
    }

    // Schedule OpenAI embedding generation if provider is configured
    if let Some(ref provider) = openai_provider {
      if let Ok(Some(album)) = app_context.album_interactor.find(&file_name).await {
        info!(
          file_name = file_name.to_string(),
          provider = provider.name(),
          "Scheduling embedding generation job"
        );
        let payload = EmbeddingGenerationJobPayload {
          provider_name: provider.name(),
          file_name: file_name.clone(),
          body: album.embedding_body(),
        };
        let params = JobParametersBuilder::default()
          .id(format!(
            "migrate_album_embedding:{}:{}",
            provider.name(),
            file_name.to_string()
          ))
          .name(provider.job_name())
          .payload(serde_json::to_vec(&payload)?)
          .build()?;
        app_context.scheduler.put(params).await?;
      }
    } else {
      warn!(
        file_name = file_name.to_string(),
        "No OpenAI provider configured, skipping embedding generation"
      );
    }
  }

  Ok(())
}

pub fn build_embedding_migration_subscriber(
  app_context: Arc<ApplicationContext>,
) -> Result<EventSubscriber> {
  Ok(
    EventSubscriberBuilder::default()
      .id("embedding_migration".to_string())
      .topic(Topic::Album)
      .batch_size(100)
      .app_context(Arc::clone(&app_context))
      .grouping_strategy(GroupingStrategy::All)
      .handler(EventHandler::Group(Arc::new(
        move |(event_data, app_context, _)| {
          Box::pin(async move {
            handle_album_saved_for_embedding_migration(event_data, app_context).await
          })
        },
      )))
      .build()?,
  )
}
