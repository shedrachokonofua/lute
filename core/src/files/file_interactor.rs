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
      file_content_store: FileContentStore::new(settings.content_store.clone()).unwrap(),
      file_metadata_repository: FileMetadataRepository::new(redis_connection_pool.get().unwrap()),
      event_publisher: EventPublisher::new(redis_connection_pool.clone()),
    }
  }

  pub fn is_file_stale(&mut self, name: String) -> Result<bool> {
    let file_name = FileName::try_from(name)?;
    let file_metadata = self.file_metadata_repository.find_by_name(&file_name)?;

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

  pub async fn put_file(
    &mut self,
    name: String,
    content: String,
    correlation_id: Option<String>,
  ) -> Result<FileMetadata> {
    let file_name = FileName::try_from(name)?;
    self.file_content_store.put(&file_name, content).await?;
    let file_metadata = self.file_metadata_repository.upsert(&file_name)?;
    self.event_publisher.publish(
      Stream::File,
      EventPayload {
        event: Event::FileSaved {
          file_id: file_metadata.id.clone(),
          file_name: file_metadata.name.clone(),
        },
        correlation_id,
        metadata: None,
      },
    )?;
    Ok(file_metadata)
  }
}
