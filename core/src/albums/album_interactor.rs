use super::{
  album_read_model::AlbumReadModel,
  album_repository::AlbumRepository,
  album_search_index::{AlbumSearchIndex, AlbumSearchQueryBuilder},
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use iter_tools::Itertools;
use std::sync::Arc;
use tracing::error;

pub struct AlbumInteractor {
  album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
}

impl AlbumInteractor {
  pub fn new(
    album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
    album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
  ) -> Self {
    Self {
      album_repository,
      album_search_index,
    }
  }

  async fn process_duplicates(&self, album: &AlbumReadModel) -> Result<()> {
    let results = self
      .album_search_index
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
          .include_duplicates(true)
          .build()?,
        None,
      )
      .await?;

    if results.total < 2 {
      return Ok(());
    }

    let mut duplicate_albums = results
      .albums
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
        .album_repository
        .set_duplicates(&original_album.file_name, duplicates.clone())
        .await?;
      self
        .album_search_index
        .set_duplicates(&original_album.file_name, duplicates)
        .await?;
    }

    for duplicate_album in duplicate_albums {
      if duplicate_album.duplicate_of != Some(original_album.file_name.clone()) {
        self
          .album_repository
          .set_duplicate_of(&duplicate_album.file_name, &original_album.file_name)
          .await?;
        self
          .album_search_index
          .set_duplicate_of(&duplicate_album.file_name, &original_album.file_name)
          .await?;
      }
    }

    Ok(())
  }

  pub async fn put(&self, album: AlbumReadModel) -> Result<()> {
    let file_name = album.file_name.clone();
    self.album_repository.put(album.clone()).await?;
    self.album_search_index.put(album.clone()).await?;
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

  async fn process_duplicates_by_file_name(&self, file_name: &FileName) -> Result<()> {
    let album = self.album_repository.get(file_name).await?;
    self.process_duplicates(&album).await
  }

  pub async fn delete(&self, file_name: &FileName) -> Result<()> {
    let album = self.album_repository.get(file_name).await?;
    self.album_repository.delete(file_name).await?;
    self.album_search_index.delete(file_name).await?;
    // If this album is a duplicate, we need to re-process the original album.
    // If this album has duplicates, we need to re-process them. It is enough to only re-process the first duplicate, as that will cascade to the rest.
    if let Some(duplicate_of) = &album.duplicate_of.as_ref().or(album.duplicates.first()) {
      if let Err(err) = self.process_duplicates_by_file_name(file_name).await {
        error!(
          "Failed to process duplicates for {}: {}",
          duplicate_of.to_string(),
          err
        );
      }
    }
    Ok(())
  }
}
