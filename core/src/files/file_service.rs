use std::sync::Arc;

use super::{
  file_interactor::FileInteractor, file_metadata::file_metadata_repository::FileMetadataRepository,
};
use crate::{
  proto::{self, IsFileStaleReply, IsFileStaleRequest, PutFileReply, PutFileRequest},
  settings::FileSettings,
};
use anyhow::Result;
use tonic::{Request, Response, Status};

pub struct FileService {
  file_settings: FileSettings,
  redis_connection_pool: Arc<r2d2::Pool<redis::Client>>,
}

impl FileService {
  pub fn new(
    file_settings: FileSettings,
    redis_connection_pool: Arc<r2d2::Pool<redis::Client>>,
  ) -> Self {
    Self {
      file_settings,
      redis_connection_pool,
    }
  }
}

#[tonic::async_trait]
impl proto::FileService for FileService {
  async fn is_file_stale(
    &self,
    request: Request<IsFileStaleRequest>,
  ) -> Result<Response<IsFileStaleReply>, Status> {
    let mut file_interactor = FileInteractor::new(
      self.file_settings.clone(),
      self.redis_connection_pool.get().unwrap(),
    );
    let name = request.into_inner().name;
    let stale = file_interactor.is_file_stale(name).map_err(|e| {
      println!("Error: {:?}", e);
      Status::internal("Internal server error")
    })?;

    let reply = IsFileStaleReply { stale };
    Ok(Response::new(reply))
  }

  async fn put_file(
    &self,
    request: Request<PutFileRequest>,
  ) -> Result<Response<PutFileReply>, Status> {
    let mut file_interactor = FileInteractor::new(
      self.file_settings.clone(),
      self.redis_connection_pool.get().unwrap(),
    );
    let inner = request.into_inner();
    let file_metadata = file_interactor
      .put_file(inner.name.clone(), &inner.content, Some("id".to_string()))
      .await
      .map_err(|e| {
        println!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?;

    let reply = PutFileReply {
      metadata: Some(file_metadata.into()),
    };
    Ok(Response::new(reply))
  }
}
