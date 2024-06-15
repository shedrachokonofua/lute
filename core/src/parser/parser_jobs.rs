use super::parse::parse_file_on_store;
use crate::{
  context::ApplicationContext,
  files::file_metadata::file_name::FileName,
  job_executor,
  scheduler::{
    job_name::JobName,
    scheduler::{JobExecutorFn, JobProcessorBuilder},
    scheduler_repository::Job,
  },
};
use anyhow::Result;
use std::sync::Arc;
use tracing::error;

async fn retry_parse(job: Job, app_context: Arc<ApplicationContext>) -> Result<()> {
  let file_name = job.payload::<FileName>()?;

  let file_metadata = app_context
    .file_interactor
    .get_file_metadata(&file_name)
    .await
    .inspect_err(|e| error!(err = e.to_string(), "Failed to get file metadata"))?;

  parse_file_on_store(
    Arc::clone(&app_context),
    file_metadata.id,
    file_name,
    Some(format!("retry:{}", job.id)),
  )
  .await
  .inspect_err(|e| error!(err = e.to_string(), "Failed to parse file"))?;

  Ok(())
}

pub async fn setup_parser_jobs(app_context: Arc<ApplicationContext>) -> Result<()> {
  app_context
    .scheduler
    .register(
      JobProcessorBuilder::default()
        .name(JobName::ParserRetry)
        .app_context(Arc::clone(&app_context))
        .executor(job_executor!(retry_parse))
        .build()?,
    )
    .await;
  Ok(())
}
