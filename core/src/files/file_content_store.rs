use super::file_metadata::file_name::FileName;
use crate::settings::ContentStoreSettings;
use anyhow::{Ok, Result};
use s3::{creds::Credentials, Bucket};
use tracing::info;

pub struct FileContentStore {
  bucket: Bucket,
}

impl FileContentStore {
  pub fn new(settings: ContentStoreSettings) -> Result<Self> {
    Ok(Self {
      bucket: Bucket::new(
        &settings.bucket,
        s3::Region::Custom {
          region: settings.region,
          endpoint: settings.endpoint,
        },
        Credentials::new(
          Some(&settings.key),
          Some(&settings.secret),
          None,
          None,
          None,
        )?,
      )?,
    })
  }

  pub async fn put(&self, file_name: &FileName, content: String) -> Result<()> {
    self
      .bucket
      .put_object(file_name.to_string(), content.as_bytes())
      .await?;
    info!(
      file_name = file_name.to_string().as_str(),
      "File saved to content store"
    );
    Ok(())
  }

  pub async fn get(&self, file_name: &FileName) -> Result<String> {
    let response = self.bucket.get_object(file_name.to_string()).await?;
    response.to_string().map_err(|e| e.into())
  }
}
