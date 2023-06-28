use super::{
  file_content_store::FileContentStore,
  file_metadata::{
    file_metadata::FileMetadata, file_metadata_repository::FileMetadataRepository,
    file_name::FileName, file_timestamp::FileTimestamp,
  },
};
use crate::{
  events::{
    event::{Event, EventPayload, Stream},
    event_publisher::EventPublisher,
  },
  settings::FileSettings,
};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tracing::info;

#[derive(Debug)]
pub struct FileInteractor {
  settings: FileSettings,
  file_content_store: FileContentStore,
  file_metadata_repository: FileMetadataRepository,
  event_publisher: EventPublisher,
}

impl FileInteractor {
  pub fn new(
    settings: FileSettings,
    redis_connection_pool: Arc<r2d2::Pool<redis::Client>>,
  ) -> Self {
    Self {
      settings: settings.clone(),
      file_content_store: FileContentStore::new(settings.content_store).unwrap(),
      file_metadata_repository: FileMetadataRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      event_publisher: EventPublisher::new(redis_connection_pool),
    }
  }

  pub fn is_file_stale(&self, file_name: &FileName) -> Result<bool> {
    let file_metadata = self.file_metadata_repository.find_by_name(file_name)?;

    Ok(
      file_metadata
        .map(|file_metadata| {
          let now: DateTime<Utc> = FileTimestamp::now().into();
          let last_saved_at: DateTime<Utc> = file_metadata.last_saved_at.into();
          let stale_at = last_saved_at + Duration::days(self.settings.ttl_days.album.into());
          now > stale_at
        })
        .unwrap_or(true),
    )
  }

  pub fn put_file_metadata(
    &self,
    file_name: &FileName,
    correlation_id: Option<String>,
  ) -> Result<FileMetadata> {
    let file_metadata = self.file_metadata_repository.upsert(&file_name)?;
    info!(file_name = file_name.to_string(), "File metadata saved");
    self.event_publisher.publish(
      Stream::File,
      EventPayload {
        event: Event::FileSaved {
          file_id: file_metadata.id,
          file_name: file_metadata.name.clone(),
        },
        correlation_id,
        metadata: None,
      },
    )?;
    Ok(file_metadata)
  }

  pub async fn put_file(
    &self,
    file_name: &FileName,
    content: String,
    correlation_id: Option<String>,
  ) -> Result<FileMetadata> {
    self.file_content_store.put(&file_name, content).await?;
    self.put_file_metadata(&file_name, correlation_id)
  }

  pub async fn list_files(&self) -> Result<Vec<FileName>> {
    self.file_content_store.list_files().await
  }
}
