use super::{
  artist_read_model::{ArtistOverview, ArtistReadModel},
  artist_repository::ArtistRepository,
  artist_search_index::{ArtistSearchIndex, ArtistSearchQuery, ArtistSearchRecord},
};
use crate::{
  albums::album_interactor::AlbumInteractor,
  files::file_metadata::file_name::FileName,
  helpers::{embedding::EmbeddingDocument, redisearch::SearchPagination},
  sqlite::SqliteConnection,
};
use anyhow::Result;
use elasticsearch::Elasticsearch;
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};
use tracing::instrument;

pub struct ArtistInteractor {
  artist_repository: ArtistRepository,
  artist_search_index: ArtistSearchIndex,
  album_interactor: Arc<AlbumInteractor>,
}

impl ArtistInteractor {
  pub fn new(
    sqlite_connection: Arc<SqliteConnection>,
    elasticsearch_client: Arc<Elasticsearch>,
    album_interactor: Arc<AlbumInteractor>,
  ) -> Self {
    Self {
      artist_repository: ArtistRepository::new(sqlite_connection),
      artist_search_index: ArtistSearchIndex::new(elasticsearch_client),
      album_interactor,
    }
  }

  #[instrument(skip(self))]
  pub async fn setup_search_index(&self) -> Result<()> {
    self.artist_search_index.setup_index().await
  }

  #[instrument(skip(self), fields(artists = artist_file_names.len()))]
  pub async fn find_many(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, ArtistReadModel>> {
    self.artist_repository.find_many(artist_file_names).await
  }

  #[instrument(skip(self))]
  pub async fn find(&self, artist_file_name: FileName) -> Result<Option<ArtistReadModel>> {
    self
      .artist_repository
      .find_many(vec![artist_file_name.clone()])
      .await
      .map(|mut artists| artists.remove(&artist_file_name))
  }

  #[instrument(skip_all, fields(artists = artists.len()))]
  async fn get_overviews_with_artist_map(
    &self,
    artists: &HashMap<FileName, ArtistReadModel>,
  ) -> Result<HashMap<FileName, ArtistOverview>> {
    let mut album_file_names = HashSet::new();
    for artist in artists.values() {
      for album_file_name in &artist.album_file_names {
        album_file_names.insert(album_file_name.clone());
      }
      for credit in &artist.credits {
        album_file_names.insert(credit.album_file_name.clone());
      }
    }
    let albums = self
      .album_interactor
      .find_many(album_file_names.into_iter().collect())
      .await?;

    let mut overviews = HashMap::new();
    for (file_name, artist) in artists {
      let overview = ArtistOverview::new(artist, &albums);
      overviews.insert(file_name.clone(), overview);
    }

    Ok(overviews)
  }

  #[instrument(skip(self), fields(artists = artist_file_names.len()))]
  pub async fn get_overviews(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, ArtistOverview>> {
    let artists = self.find_many(artist_file_names).await?;
    self.get_overviews_with_artist_map(&artists).await
  }

  #[instrument(skip(self))]
  pub async fn get_overview(&self, artist_file_name: FileName) -> Result<Option<ArtistOverview>> {
    self
      .get_overviews(vec![artist_file_name.clone()])
      .await
      .map(|mut overviews| overviews.remove(&artist_file_name))
  }

  #[instrument(skip_all, fields(artists = artist_file_names.len()))]
  pub async fn update_search_records(&self, artist_file_names: Vec<FileName>) -> Result<()> {
    let overviews = self.get_overviews(artist_file_names).await?;
    self
      .artist_search_index
      .put_many(
        overviews.into_values().map(|overview| overview.into())
          .collect::<Vec<ArtistSearchRecord>>(),
      )
      .await?;
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn search(
    &self,
    query: &ArtistSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<(Vec<(ArtistReadModel, ArtistOverview)>, usize)> {
    let result = self.artist_search_index.search(query, pagination).await?;
    let file_names = result
      .artists
      .iter()
      .filter_map(|artist| FileName::try_from(artist.file_name.clone()).ok())
      .collect();
    let mut artists = self.find_many(file_names).await?;
    let mut overviews = self.get_overviews_with_artist_map(&artists).await?;
    let output = result
      .artists
      .into_iter()
      .filter_map(|artist| {
        FileName::try_from(artist.file_name.clone())
          .ok()
          .and_then(
            |file_name| match (artists.remove(&file_name), overviews.remove(&file_name)) {
              (Some(artist), Some(overview)) => Some((artist, overview)),
              _ => None,
            },
          )
      })
      .collect();
    Ok((output, result.total))
  }

  #[instrument(skip_all, fields(count = docs.len()))]
  pub async fn put_many_embeddings(&self, docs: Vec<EmbeddingDocument>) -> Result<()> {
    self.artist_search_index.put_many_embeddings(docs).await
  }
}
