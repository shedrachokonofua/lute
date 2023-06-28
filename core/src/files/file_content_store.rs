use super::file_metadata::file_name::FileName;
use crate::settings::ContentStoreSettings;
use anyhow::Result;
use s3::{creds::Credentials, Bucket};
use tracing::{info, instrument, warn};

#[derive(Debug)]
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

  #[instrument(skip(self))]
  pub async fn get(&self, file_name: &FileName) -> Result<String> {
    let response = self.bucket.get_object(file_name.to_string()).await?;
    response.to_string().map_err(|e| e.into())
  }

  #[instrument(skip(self))]
  pub async fn list_files(&self) -> Result<Vec<FileName>> {
    let mut objects = self.bucket.list("release/".to_string(), None).await?;
    objects.append(&mut self.bucket.list("charts/".to_string(), None).await?);
    objects.append(&mut self.bucket.list("artist/".to_string(), None).await?);

    Ok(
      objects
        .into_iter()
        .flat_map(|page| {
          page
            .contents
            .into_iter()
            .map(|object| FileName::try_from(object.key))
            .collect::<Vec<Result<FileName>>>()
        })
        .filter_map(|o| match o {
          Ok(file_name) => Some(file_name),
          Err(e) => {
            warn!("Invalid file name: {:?}", e);
            None
          }
        })
        .collect(),
    )
  }
}
