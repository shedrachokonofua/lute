use super::{
  artist_read_model::{ArtistOverview, ArtistReadModel},
  artist_repository::ArtistRepository,
  artist_search_index::{ArtistSearchIndex, ArtistSearchQuery},
};
use crate::{
  albums::album_interactor::AlbumInteractor, files::file_metadata::file_name::FileName,
  helpers::redisearch::SearchPagination, sqlite::SqliteConnection,
};
use anyhow::Result;
use futures::future::join_all;
use rustis::{bb8::Pool, client::PooledClientManager};
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
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    album_interactor: Arc<AlbumInteractor>,
  ) -> Self {
    Self {
      artist_repository: ArtistRepository::new(sqlite_connection),
      artist_search_index: ArtistSearchIndex::new(redis_connection_pool),
      album_interactor,
    }
  }

  #[instrument(skip(self))]
  pub async fn setup_search_index(&self) -> Result<()> {
    self.artist_search_index.setup_index().await
  }

  #[instrument(skip(self))]
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

  #[instrument(skip(self))]
  pub async fn get_overviews(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, ArtistOverview>> {
    let artists = self.find_many(artist_file_names).await?;

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
      overviews.insert(file_name, overview);
    }

    Ok(overviews)
  }

  #[instrument(skip(self))]
  pub async fn get_overview(&self, artist_file_name: FileName) -> Result<Option<ArtistOverview>> {
    self
      .get_overviews(vec![artist_file_name.clone()])
      .await
      .map(|mut overviews| overviews.remove(&artist_file_name))
  }

  #[instrument(skip(self))]
  pub async fn update_search_records(&self, artist_file_names: Vec<FileName>) -> Result<()> {
    let overviews = self.get_overviews(artist_file_names).await?;
    join_all(
      overviews
        .into_iter()
        .map(|(_, overview)| self.artist_search_index.put(overview)),
    )
    .await;
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn search(
    &self,
    query: &ArtistSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<(Vec<ArtistOverview>, usize)> {
    let result = self.artist_search_index.search(query, pagination).await?;
    let overviews = self
      .get_overviews(
        result
          .artists
          .iter()
          .filter_map(|artist: &super::artist_search_index::ArtistSearchRecord| {
            FileName::try_from(artist.file_name.clone()).ok()
          })
          .collect(),
      )
      .await?;
    Ok((overviews.values().cloned().collect(), result.total))
  }
}
