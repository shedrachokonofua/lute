use super::album_repository::{AlbumRepository, AlbumSearchQuery};
use crate::{files::file_metadata::file_name::FileName, proto};
use anyhow::{Error, Result};
use std::sync::Arc;
use tonic::{async_trait, Request, Response, Status};

pub struct AlbumService {
  album_read_model_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
}

impl AlbumService {
  pub fn new(
    album_read_model_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  ) -> Self {
    Self {
      album_read_model_repository,
    }
  }
}

#[async_trait]
impl proto::AlbumService for AlbumService {
  async fn get_album(
    &self,
    request: Request<proto::GetAlbumRequest>,
  ) -> Result<Response<proto::GetAlbumReply>, Status> {
    let file_name = FileName::try_from(request.into_inner().file_name)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let album = self
      .album_read_model_repository
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
      .album_read_model_repository
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
      .album_read_model_repository
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
        .album_read_model_repository
        .get_embedding_keys()
        .await
        .map_err(|e| Status::internal(e.to_string()))?,
    };
    Ok(Response::new(reply))
  }

  async fn get_aggregated_genres(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetAggregatedGenresReply>, Status> {
    let reply = proto::GetAggregatedGenresReply {
      genres: self
        .album_read_model_repository
        .get_aggregated_genres()
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .iter()
        .map(|i| i.into())
        .collect(),
    };
    Ok(Response::new(reply))
  }

  async fn get_aggregated_descriptors(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetAggregatedDescriptorsReply>, Status> {
    let reply = proto::GetAggregatedDescriptorsReply {
      descriptors: self
        .album_read_model_repository
        .get_aggregated_descriptors()
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .iter()
        .map(|i| i.into())
        .collect(),
    };
    Ok(Response::new(reply))
  }

  async fn get_aggregated_languages(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetAggregatedLanguagesReply>, Status> {
    let reply = proto::GetAggregatedLanguagesReply {
      languages: self
        .album_read_model_repository
        .get_aggregated_languages()
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .iter()
        .map(|i| i.into())
        .collect(),
    };
    Ok(Response::new(reply))
  }
}
