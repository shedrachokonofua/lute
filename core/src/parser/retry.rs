use crate::{
  context::ApplicationContext, files::file_content_store::FileContentStore,
  parser::parser::parse_file_on_store,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::spawn;
use tracing::{error, info};
use ulid::Ulid;

pub fn start_parser_retry_consumer(app_context: Arc<ApplicationContext>) -> Result<()> {
  let file_content_store = FileContentStore::new(&app_context.settings.file.content_store)?;

  spawn(async move {
    loop {
      match app_context.parser_retry_queue.recv().await {
        Ok(file_name) => {
          info!(
            file_name = file_name.to_string().as_str(),
            "Retrying file parse"
          );

          match app_context
            .file_interactor
            .get_file_metadata(&file_name)
            .await
          {
            Ok(file_metadata) => {
              if let Err(e) = parse_file_on_store(
                file_content_store.clone(),
                Arc::clone(&app_context.event_publisher),
                file_metadata.id,
                file_name,
                Some(format!("retry:{}", Ulid::new().to_string())),
              )
              .await
              {
                error!(
                  error = e.to_string().as_str(),
                  "Failed to parse file on store"
                );
              }
            }
            Err(e) => {
              error!(
                error = e.to_string().as_str(),
                "Failed to get file metadata"
              );
            }
          }
        }
        Err(error) => {
          error!(
            error = error.to_string().as_str(),
            "Failed to retry file parse"
          );
        }
      }
    }
  });

  Ok(())
}
