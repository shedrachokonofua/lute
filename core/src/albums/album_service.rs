use super::album_read_model_repository::AlbumReadModelRepository;
use crate::{files::file_metadata::file_name::FileName, proto};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tonic::Status;

pub struct AlbumService {
  album_read_model_repository: AlbumReadModelRepository,
}

impl AlbumService {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      album_read_model_repository: AlbumReadModelRepository {
        redis_connection_pool,
      },
    }
  }
}

#[tonic::async_trait]
impl proto::AlbumService for AlbumService {
  async fn get_album(
    &self,
    request: tonic::Request<proto::GetAlbumRequest>,
  ) -> Result<tonic::Response<proto::GetAlbumReply>, tonic::Status> {
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
    Ok(tonic::Response::new(reply))
  }
}
