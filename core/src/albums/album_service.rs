use std::sync::Arc;

use super::album_read_model_repository::AlbumReadModelRepository;
use crate::{files::file_metadata::file_name::FileName, proto};
use tonic::{async_trait, Request, Response, Status};

pub struct AlbumService {
  album_read_model_repository: Arc<dyn AlbumReadModelRepository + Send + Sync + 'static>,
}

impl AlbumService {
  pub fn new(
    album_read_model_repository: Arc<dyn AlbumReadModelRepository + Send + Sync + 'static>,
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
