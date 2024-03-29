use super::{
  album_read_model::AlbumReadModel,
  album_repository::{AlbumRepository, GenreAggregate, ItemAndCount},
  album_search_index::AlbumSearchIndex,
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use iter_tools::Itertools;
use std::sync::Arc;
use tokio::try_join;
use tracing::{error, instrument};

pub struct AlbumMonitor {
  pub album_count: u32,
  pub artist_count: u32,
  pub genre_count: u32,
  pub descriptor_count: u32,
  pub duplicate_count: u32,
  pub language_count: u32,
  pub aggregated_genres: Vec<GenreAggregate>,
  pub aggregated_descriptors: Vec<ItemAndCount>,
  pub aggregated_languages: Vec<ItemAndCount>,
  pub aggregated_years: Vec<ItemAndCount>,
}

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

  #[instrument(skip(self))]
  pub async fn get_monitor(&self) -> Result<AlbumMonitor> {
    let (
      album_count,
      artist_count,
      genre_count,
      descriptor_count,
      language_count,
      duplicate_count,
      aggregated_genres,
      aggregated_descriptors,
      aggregated_languages,
      aggregated_years,
    ) = try_join!(
      self.album_repository.get_album_count(),
      self.album_repository.get_artist_count(),
      self.album_repository.get_genre_count(),
      self.album_repository.get_descriptor_count(),
      self.album_repository.get_language_count(),
      self.album_repository.get_duplicate_count(),
      self.album_repository.get_aggregated_genres(None),
      self.album_repository.get_aggregated_descriptors(None),
      self.album_repository.get_aggregated_languages(None),
      self.album_repository.get_aggregated_years(None)
    )?;
    Ok(AlbumMonitor {
      album_count,
      artist_count,
      genre_count,
      descriptor_count,
      duplicate_count,
      language_count,
      aggregated_genres,
      aggregated_descriptors,
      aggregated_languages,
      aggregated_years,
    })
  }

  #[instrument(skip(self))]
  async fn process_duplicates(&self, album: &AlbumReadModel) -> Result<()> {
    let potential_duplicates = self
      .album_repository
      .find_artist_albums(
        album
          .artists
          .iter()
          .map(|artist| artist.file_name.clone())
          .collect(),
      )
      .await?
      .into_iter()
      .filter(|potential_duplicate| {
        potential_duplicate
          .ascii_name()
          .eq_ignore_ascii_case(album.ascii_name().as_str())
      })
      .collect::<Vec<_>>();

    if potential_duplicates.len() <= 1 {
      return Ok(());
    }

    let mut duplicate_albums = potential_duplicates
      .into_iter()
      .sorted_by(|a, b| {
        b.rating_count
          .partial_cmp(&a.rating_count)
          .unwrap_or(std::cmp::Ordering::Equal)
      })
      .collect::<Vec<_>>();
    let mut original_album = duplicate_albums.remove(0);

    let mut duplicates = duplicate_albums
      .iter()
      .map(|album| album.file_name.clone())
      .collect::<Vec<FileName>>();
    duplicates.sort();

    let original_album_file_name = original_album.file_name.clone();
    if original_album.duplicates != duplicates {
      self
        .album_repository
        .set_duplicates(&original_album.file_name, duplicates.clone())
        .await?;
      original_album.duplicates = duplicates;
      self.album_search_index.put(original_album).await?;
    }

    for mut duplicate_album in duplicate_albums.into_iter() {
      if duplicate_album
        .duplicate_of
        .as_ref()
        .map(|d| d != &original_album_file_name)
        .unwrap_or(true)
      {
        self
          .album_repository
          .set_duplicate_of(&duplicate_album.file_name, &original_album_file_name)
          .await?;
        duplicate_album.duplicate_of = Some(original_album_file_name.clone());
        self.album_search_index.put(duplicate_album).await?;
      }
    }

    Ok(())
  }

  #[instrument(skip(self), name = "AlbumInteractor::put")]
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
