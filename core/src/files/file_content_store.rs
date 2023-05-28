use crate::settings::ContentStoreSettings;
use anyhow::{Ok, Result};
use s3::{creds::Credentials, Bucket};

use super::file_metadata::file_name::FileName;

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

  pub async fn put(&self, id: &FileName, content: &str) -> Result<()> {
    self
      .bucket
      .put_object(id.to_string(), content.as_bytes())
      .await?;
    Ok(())
  }
}
