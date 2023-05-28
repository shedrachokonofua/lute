use super::file_metadata::{
  file_metadata::FileMetadata, file_metadata_repository::FileMetadataRepository,
  file_name::FileName, file_timestamp::FileTimestamp,
};
use crate::settings::FileSettings;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

pub struct FileInteractor {
  settings: FileSettings,
  file_metadata_repository: FileMetadataRepository,
}

impl FileInteractor {
  pub fn new(settings: FileSettings, file_metadata_repository: FileMetadataRepository) -> Self {
    Self {
      settings,
      file_metadata_repository,
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

  pub fn put_file(
    &mut self,
    name: String,
    content: &str,
    correlation_id: Option<String>,
  ) -> Result<FileMetadata> {
    let file_name = FileName::try_from(name)?;
    self.file_metadata_repository.upsert(&file_name)
  }
}
