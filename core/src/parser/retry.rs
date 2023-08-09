use crate::{
  events::event_publisher::EventPublisher,
  files::{
    file_content_store::FileContentStore, file_interactor::FileInteractor,
    file_metadata::file_name::FileName,
  },
  helpers::fifo_queue::FifoQueue,
  parser::parser::parse_file_on_store,
  settings::Settings,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tokio::spawn;
use tracing::{error, info};
use ulid::Ulid;

pub fn start_parser_retry_consumer(
  queue: Arc<FifoQueue<FileName>>,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Arc<Settings>,
) -> Result<()> {
  let file_content_store = FileContentStore::new(&settings.file.content_store)?;
  let event_publisher = EventPublisher::new(Arc::clone(&redis_connection_pool));
  let file_interactor =
    FileInteractor::new(Arc::clone(&settings), Arc::clone(&redis_connection_pool));

  spawn(async move {
    loop {
      match queue.recv().await {
        Ok(file_name) => {
          info!(
            file_name = file_name.to_string().as_str(),
            "Retrying file parse"
          );

          match file_interactor.get_file_metadata(&file_name).await {
            Ok(file_metadata) => {
              if let Err(e) = parse_file_on_store(
                file_content_store.clone(),
                event_publisher.clone(),
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
