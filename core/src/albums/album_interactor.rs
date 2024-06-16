use super::{
  album_read_model::AlbumReadModel,
  album_repository::{AlbumRepository, GenreAggregate, ItemAndCount},
  album_search_index::{
    AlbumEmbeddingSimilarirtySearchQuery, AlbumSearchIndex, AlbumSearchQuery, AlbumSearchResult,
  },
};
use crate::{
  events::{
    event::{Event, EventPayloadBuilder, Topic},
    event_publisher::EventPublisher,
  },
  files::file_metadata::file_name::FileName,
  helpers::{embedding::EmbeddingDocument, redisearch::SearchPagination},
};
use anyhow::Result;
use iter_tools::Itertools;
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};
use tokio::try_join;
use tracing::{error, instrument};

pub struct AlbumMonitor {
  pub album_count: u32,
  pub artist_count: u32,
  pub genre_count: u32,
  pub descriptor_count: u32,
  pub duplicate_count: u32,
  pub language_count: u32,
  pub spotify_id_count: u32,
  pub aggregated_genres: Vec<GenreAggregate>,
  pub aggregated_descriptors: Vec<ItemAndCount>,
  pub aggregated_languages: Vec<ItemAndCount>,
  pub aggregated_years: Vec<ItemAndCount>,
}

pub struct AlbumInteractor {
  album_repository: Arc<AlbumRepository>,
  album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
  event_publisher: Arc<EventPublisher>,
}

impl AlbumInteractor {
  pub fn new(
    album_repository: Arc<AlbumRepository>,
    album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
    event_publisher: Arc<EventPublisher>,
  ) -> Self {
    Self {
      album_repository,
      album_search_index,
      event_publisher,
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
      spotify_id_count,
      aggregated_genres,
      aggregated_descriptors,
      aggregated_languages,
      aggregated_years,
    ) = try_join!(
      self.album_repository.count_albums(),
      self.album_repository.count_artists(),
      self.album_repository.count_genres(),
      self.album_repository.count_descriptors(),
      self.album_repository.count_languages(),
      self.album_repository.count_duplicates(),
      self.album_repository.count_spotify_ids(),
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
      spotify_id_count,
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

  #[instrument(skip_all, name = "AlbumInteractor::put_many", fields(count = albums.len()))]
  pub async fn put_many(&self, albums: Vec<AlbumReadModel>) -> Result<()> {
    let album_file_names = albums
      .iter()
      .map(|album| album.file_name.clone())
      .collect::<Vec<_>>();
    self.album_repository.put_many(albums.clone()).await?;
    for album in albums.iter() {
      if let Err(err) = self.album_search_index.put(album.clone()).await {
        error!(
          "Failed to put album into search index {}: {}",
          album.file_name.to_string(),
          err
        );
      }
      if let Err(err) = self.process_duplicates(album).await {
        error!(
          "Failed to process duplicates for {}: {}",
          album.file_name.to_string(),
          err
        );
      }
    }
    self
      .event_publisher
      .publish_many(
        Topic::Album,
        album_file_names
          .into_iter()
          .map(|file_name| {
            Ok(
              EventPayloadBuilder::default()
                .key(file_name.clone())
                .event(Event::AlbumSaved { file_name })
                .build()?,
            )
          })
          .collect::<Result<Vec<_>>>()?,
      )
      .await?;
    Ok(())
  }

  #[instrument(skip(self), name = "AlbumInteractor::put")]
  pub async fn put(&self, album: AlbumReadModel) -> Result<()> {
    self.put_many(vec![album]).await
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

  pub async fn find_many(
    &self,
    album_file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, AlbumReadModel>> {
    let albums = self.album_repository.find_many(album_file_names).await?;
    Ok(
      albums
        .into_iter()
        .map(|album| (album.file_name.clone(), album))
        .collect(),
    )
  }

  pub async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    self.album_repository.find(file_name).await
  }

  pub async fn get(&self, file_name: &FileName) -> Result<AlbumReadModel> {
    self.album_repository.get(file_name).await
  }

  pub async fn get_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>> {
    self.album_repository.get_many(file_names).await
  }

  pub async fn search(
    &self,
    query: &AlbumSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<AlbumSearchResult> {
    self.album_search_index.search(query, pagination).await
  }

  pub async fn find_many_embeddings(
    &self,
    file_names: Vec<FileName>,
    key: &str,
  ) -> Result<Vec<EmbeddingDocument>> {
    self
      .album_search_index
      .find_many_embeddings(file_names, key)
      .await
  }

  pub async fn find_embedding(
    &self,
    file_name: &FileName,
    key: &str,
  ) -> Result<Option<EmbeddingDocument>> {
    self.album_search_index.find_embedding(file_name, key).await
  }

  pub async fn embedding_similarity_search(
    &self,
    query: &AlbumEmbeddingSimilarirtySearchQuery,
  ) -> Result<Vec<(AlbumReadModel, f32)>> {
    self
      .album_search_index
      .embedding_similarity_search(query)
      .await
  }

  pub async fn put_embedding(&self, embedding: &EmbeddingDocument) -> Result<()> {
    self.album_search_index.put_embedding(embedding).await
  }

  pub async fn put_many_embeddings(&self, embeddings: Vec<EmbeddingDocument>) -> Result<()> {
    for embedding in embeddings {
      self.album_search_index.put_embedding(&embedding).await?;
    }
    Ok(())
  }

  pub async fn related_artist_file_names(
    &self,
    album_file_names: Vec<FileName>,
  ) -> Result<Vec<FileName>> {
    let albums = self.find_many(album_file_names).await?;
    let artist_file_names = albums
      .values()
      .flat_map(|album| {
        album
          .credits
          .iter()
          .map(|credit| credit.artist.file_name.clone())
          .chain(album.artists.iter().map(|artist| artist.file_name.clone()))
      })
      .collect::<HashSet<_>>()
      .into_iter()
      .collect::<Vec<_>>();
    Ok(artist_file_names)
  }

  #[instrument(skip(self))]
  pub async fn find_similar_albums(
    &self,
    file_name: FileName,
    embedding_key: &str,
    filters: Option<AlbumSearchQuery>,
    limit: usize,
  ) -> Result<Vec<AlbumReadModel>> {
    let embedding = self
      .find_embedding(&file_name, embedding_key)
      .await?
      .ok_or_else(|| anyhow::anyhow!("Embedding not found"))?;

    let mut filters = filters.unwrap_or_default();
    if !filters.exclude_file_names.contains(&file_name) {
      filters.exclude_file_names.push(file_name);
    }

    let query = AlbumEmbeddingSimilarirtySearchQuery {
      embedding: embedding.embedding,
      embedding_key: embedding_key.to_string(),
      filters,
      limit,
    };

    self
      .embedding_similarity_search(&query)
      .await
      .map(|results| results.into_iter().map(|(album, _)| album).collect())
  }
}
