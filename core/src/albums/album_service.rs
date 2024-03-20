use super::{
  album_interactor::{AlbumInteractor, AlbumMonitor},
  album_repository::{AlbumRepository, GenreAggregate, ItemAndCount},
  album_search_index::{
    AlbumEmbeddingSimilarirtySearchQuery, AlbumSearchIndex, AlbumSearchQuery, SearchPagination,
  },
};
use crate::{files::file_metadata::file_name::FileName, proto};
use anyhow::{Error, Result};
use std::sync::Arc;
use tonic::{async_trait, Request, Response, Status};

impl From<GenreAggregate> for proto::GenreAggregate {
  fn from(val: GenreAggregate) -> Self {
    proto::GenreAggregate {
      name: val.name,
      primary_genre_count: val.primary_genre_count,
      secondary_genre_count: val.secondary_genre_count,
    }
  }
}

impl From<AlbumMonitor> for proto::AlbumMonitor {
  fn from(val: AlbumMonitor) -> Self {
    proto::AlbumMonitor {
      album_count: val.album_count,
      artist_count: val.artist_count,
      genre_count: val.genre_count,
      descriptor_count: val.descriptor_count,
      duplicate_count: val.duplicate_count,
      language_count: val.language_count,
      aggregated_genres: val
        .aggregated_genres
        .into_iter()
        .map(|i| i.into())
        .collect(),
      aggregated_descriptors: val
        .aggregated_descriptors
        .into_iter()
        .map(|i| i.into())
        .collect(),
      aggregated_languages: val
        .aggregated_languages
        .into_iter()
        .map(|i| i.into())
        .collect(),
      aggregated_years: val.aggregated_years.into_iter().map(|i| i.into()).collect(),
    }
  }
}

impl From<ItemAndCount> for proto::ItemAndCount {
  fn from(val: ItemAndCount) -> Self {
    proto::ItemAndCount {
      name: val.name,
      count: val.count,
    }
  }
}

impl TryFrom<proto::AlbumSearchQuery> for AlbumSearchQuery {
  type Error = anyhow::Error;

  fn try_from(value: proto::AlbumSearchQuery) -> Result<Self> {
    Ok(AlbumSearchQuery {
      text: value.text,
      exact_name: value.exact_name,
      include_file_names: value
        .include_file_names
        .into_iter()
        .map(|file_name| FileName::try_from(file_name).map_err(|e| anyhow::Error::msg(e)))
        .collect::<Result<Vec<FileName>>>()?,
      exclude_file_names: value
        .exclude_file_names
        .into_iter()
        .map(|file_name| FileName::try_from(file_name).map_err(|e| anyhow::Error::msg(e)))
        .collect::<Result<Vec<FileName>>>()?,
      include_artists: value.include_artists,
      exclude_artists: value.exclude_artists,
      include_primary_genres: value.include_primary_genres,
      exclude_primary_genres: value.exclude_primary_genres,
      include_secondary_genres: value.include_secondary_genres,
      exclude_secondary_genres: value.exclude_secondary_genres,
      include_languages: value.include_languages,
      exclude_languages: value.exclude_languages,
      include_descriptors: value.include_descriptors,
      min_primary_genre_count: value.min_primary_genre_count.map(|i| i as usize),
      min_secondary_genre_count: value.min_secondary_genre_count.map(|i| i as usize),
      min_descriptor_count: value.min_descriptor_count.map(|i| i as usize),
      min_release_year: value.min_release_year.map(|i| i as u32),
      max_release_year: value.max_release_year.map(|i| i as u32),
      include_duplicates: value.include_duplicates,
    })
  }
}

impl TryFrom<proto::SearchPagination> for SearchPagination {
  type Error = anyhow::Error;

  fn try_from(value: proto::SearchPagination) -> Result<Self> {
    Ok(SearchPagination {
      offset: value.offset.map(|i| i as usize),
      limit: value.limit.map(|i| i as usize),
    })
  }
}

pub struct AlbumService {
  album_interactor: Arc<AlbumInteractor>,
  album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
}

impl AlbumService {
  pub fn new(
    album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
    album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
  ) -> Self {
    Self {
      album_repository: Arc::clone(&album_repository),
      album_search_index: Arc::clone(&album_search_index),
      album_interactor: Arc::new(AlbumInteractor::new(album_repository, album_search_index)),
    }
  }
}

#[async_trait]
impl proto::AlbumService for AlbumService {
  async fn get_monitor(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetAlbumMonitorReply>, Status> {
    let monitor = self
      .album_interactor
      .get_monitor()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::GetAlbumMonitorReply {
      monitor: Some(monitor.into()),
    };
    Ok(Response::new(reply))
  }

  async fn get_album(
    &self,
    request: Request<proto::GetAlbumRequest>,
  ) -> Result<Response<proto::GetAlbumReply>, Status> {
    let file_name = FileName::try_from(request.into_inner().file_name)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let album = self
      .album_repository
      .get(&file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::GetAlbumReply {
      album: Some(album.into()),
    };
    Ok(Response::new(reply))
  }

  async fn get_many_albums(
    &self,
    request: Request<proto::GetManyAlbumsRequest>,
  ) -> Result<Response<proto::GetManyAlbumsReply>, Status> {
    let file_names = request
      .into_inner()
      .file_names
      .into_iter()
      .map(|file_name| FileName::try_from(file_name))
      .collect::<Result<Vec<FileName>, Error>>()
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let albums = self
      .album_repository
      .get_many(file_names)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::GetManyAlbumsReply {
      albums: albums.into_iter().map(|album| album.into()).collect(),
    };
    Ok(Response::new(reply))
  }

  async fn search_albums(
    &self,
    request: Request<proto::SearchAlbumsRequest>,
  ) -> Result<Response<proto::SearchAlbumsReply>, Status> {
    let request = request.into_inner();
    let query: AlbumSearchQuery = request
      .query
      .map(|q| q.try_into())
      .transpose()
      .map_err(|e: Error| Status::invalid_argument(format!("Invalid query: {}", e.to_string())))?
      .unwrap_or_default();
    let pagination = request
      .pagination
      .map(|p| p.try_into())
      .transpose()
      .map_err(|e: Error| {
        Status::invalid_argument(format!("Invalid pagination: {}", e.to_string()))
      })?;
    let results = self
      .album_search_index
      .search(&query, pagination.as_ref())
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::SearchAlbumsReply {
      albums: results
        .albums
        .into_iter()
        .map(|album| album.into())
        .collect::<Vec<proto::Album>>(),
      total: results.total as u32,
    };
    Ok(Response::new(reply))
  }

  async fn get_embedding_keys(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetEmbeddingKeysReply>, Status> {
    let reply = proto::GetEmbeddingKeysReply {
      keys: self
        .album_search_index
        .get_embedding_keys()
        .await
        .map_err(|e| Status::internal(e.to_string()))?,
    };
    Ok(Response::new(reply))
  }

  async fn find_similar_albums(
    &self,
    request: Request<proto::FindSimilarAlbumsRequest>,
  ) -> Result<Response<proto::FindSimilarAlbumsReply>, Status> {
    let inner = request.into_inner();
    let file_name =
      FileName::try_from(inner.file_name).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let embedding_key = inner.embedding_key;
    let limit = inner.limit.unwrap_or(10) as usize;
    let mut filters: AlbumSearchQuery = inner
      .filters
      .map(|f| f.try_into())
      .transpose()
      .map_err(|e: Error| Status::invalid_argument(format!("Invalid filters: {}", e.to_string())))?
      .unwrap_or_default();

    if !filters.exclude_file_names.contains(&file_name) {
      filters.exclude_file_names.push(file_name.clone());
    }

    let embedding = self
      .album_search_index
      .find_embedding(&file_name, &embedding_key)
      .await
      .map_err(|e| Status::internal(e.to_string()))?
      .ok_or_else(|| Status::not_found("Album embedding not found"))?
      .embedding;

    let results = self
      .album_search_index
      .embedding_similarity_search(&AlbumEmbeddingSimilarirtySearchQuery {
        embedding,
        embedding_key,
        filters,
        limit,
      })
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::FindSimilarAlbumsReply {
      albums: results.into_iter().map(|(album, _)| album.into()).collect(),
    };
    Ok(Response::new(reply))
  }
}
