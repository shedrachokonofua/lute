use super::album_repository::{AlbumReadModel, AlbumRepository, AlbumSearchQueryBuilder};
use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use iter_tools::Itertools;
use std::sync::Arc;
use tracing::error;

pub struct AlbumInteractor {
  album_read_model_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
}

impl AlbumInteractor {
  pub fn new(
    album_read_model_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  ) -> Self {
    Self {
      album_read_model_repository: album_read_model_repository,
    }
  }

  async fn process_duplicates(&self, album: &AlbumReadModel) -> Result<()> {
    let potential_duplicates = self
      .album_read_model_repository
      .search(
        &AlbumSearchQueryBuilder::default()
          .exact_name(album.name.clone())
          .include_artists(
            album
              .artists
              .iter()
              .map(|artist| artist.file_name.to_string())
              .collect::<Vec<String>>(),
          )
          .build()?,
      )
      .await?;

    if potential_duplicates.len() < 2 {
      return Ok(());
    }

    let mut duplicate_albums = potential_duplicates
      .iter()
      .sorted_by(|a, b| {
        b.rating_count
          .partial_cmp(&a.rating_count)
          .unwrap_or(std::cmp::Ordering::Equal)
      })
      .collect::<Vec<&AlbumReadModel>>();
    let original_album = duplicate_albums.remove(0);

    let mut duplicates = duplicate_albums
      .iter()
      .map(|album| album.file_name.clone())
      .collect::<Vec<FileName>>();
    duplicates.sort();

    if original_album.duplicates != duplicates {
      self
        .album_read_model_repository
        .set_duplicates(&original_album.file_name, duplicates)
        .await?;
    }

    for duplicate_album in duplicate_albums {
      if duplicate_album.duplicate_of != Some(original_album.file_name.clone()) {
        self
          .album_read_model_repository
          .set_duplicate_of(&duplicate_album.file_name, &original_album.file_name)
          .await?;
      }
    }

    Ok(())
  }

  pub async fn put(&self, album: AlbumReadModel) -> Result<()> {
    let file_name = album.file_name.clone();
    self.album_read_model_repository.put(album.clone()).await?;
    match self.process_duplicates(&album).await {
      Ok(_) => Ok(()),
      Err(err) => {
        error!(
          "Failed to process duplicates for {}: {}",
          file_name.to_string(),
          err
        );
        Ok(())
      }
    }
  }
}
