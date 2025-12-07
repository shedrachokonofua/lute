use crate::{
  batch_job_executor,
  context::ApplicationContext,
  files::file_metadata::{file_name::FileName, page_type::PageType},
  helpers::embedding::EmbeddingDocument,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobProcessorBuilder},
    scheduler_repository::Job,
  },
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, instrument};

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingGenerationJobPayload {
  pub provider_name: String,
  pub file_name: FileName,
  pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingDeletionJobPayload {
  pub file_name: FileName,
  pub key: String,
}

#[instrument(skip_all, fields(count = jobs.len(), job_name = jobs.first().map(|j| j.name.to_string())))]
async fn generate_embeddings(jobs: Vec<Job>, app_context: Arc<ApplicationContext>) -> Result<()> {
  let payloads = jobs
    .into_iter()
    .filter_map(|job| {
      job
        .payload::<EmbeddingGenerationJobPayload>()
        .inspect_err(|e| {
          error!(
            e = e.to_string(),
            "Failed to get embedding generation job payload"
          );
        })
        .ok()
    })
    .collect::<Vec<_>>();

  let provider_name = payloads[0].provider_name.clone();
  let input = payloads
    .into_iter()
    .map(|payload| (payload.file_name, payload.body))
    .collect::<HashMap<FileName, String>>();

  let embeddings = app_context
    .embedding_provider_interactor
    .generate(&provider_name, input)
    .await?;

  let mut artist_embeddings = Vec::new();
  let mut album_embeddings = Vec::new();
  for (file_name, embedding) in embeddings {
    let doc = EmbeddingDocument {
      file_name: file_name.clone(),
      key: provider_name.clone(),
      embedding,
    };
    if file_name.page_type() == PageType::Artist {
      artist_embeddings.push(doc);
    } else if file_name.page_type() == PageType::Album {
      album_embeddings.push(doc);
    }
  }

  if !artist_embeddings.is_empty() {
    app_context
      .artist_interactor
      .put_many_embeddings(artist_embeddings)
      .await?;
  }

  if !album_embeddings.is_empty() {
    app_context
      .album_interactor
      .put_many_embeddings(album_embeddings)
      .await?;
  }

  Ok(())
}

#[instrument(skip_all, fields(count = jobs.len()))]
async fn delete_embeddings(jobs: Vec<Job>, app_context: Arc<ApplicationContext>) -> Result<()> {
  let payloads = jobs
    .into_iter()
    .filter_map(|job| {
      job
        .payload::<EmbeddingDeletionJobPayload>()
        .inspect_err(|e| {
          error!(
            e = e.to_string(),
            "Failed to get embedding deletion job payload"
          );
        })
        .ok()
    })
    .collect::<Vec<_>>();

  for payload in payloads {
    let file_name = &payload.file_name;
    let key = &payload.key;

    if file_name.page_type() == PageType::Artist {
      app_context
        .artist_interactor
        .delete_embedding(file_name, key)
        .await?;
    } else if file_name.page_type() == PageType::Album {
      app_context
        .album_interactor
        .delete_embedding(file_name, key)
        .await?;
    }
  }

  Ok(())
}

pub async fn setup_embedding_provider_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  for provider in app_context.embedding_provider_interactor.providers.values() {
    app_context
      .scheduler
      .register(
        JobProcessorBuilder::default()
          .name(provider.job_name())
          .concurrency(provider.concurrency() as u32)
          .app_context(Arc::clone(&app_context))
          .cooldown(provider.interval())
          .executor(batch_job_executor!(
            generate_embeddings,
            provider.batch_size() as u32
          ))
          .build()?,
      )
      .await;
  }

  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::DeleteEmbeddings)
        .concurrency(1)
        .app_context(Arc::clone(&app_context))
        .executor(batch_job_executor!(delete_embeddings, 100))
        .build()?,
    )
    .await;

  Ok(())
}
