use super::{file_interactor::FileInteractor, file_metadata::file_name::FileName};
use crate::proto::{self, IsFileStaleReply, IsFileStaleRequest, PutFileReply, PutFileRequest};
use anyhow::Result;
use tonic::{Request, Response, Status};
use tracing::error;

pub struct FileService {
  pub file_interactor: FileInteractor,
}

#[tonic::async_trait]
impl proto::FileService for FileService {
  async fn is_file_stale(
    &self,
    request: Request<IsFileStaleRequest>,
  ) -> Result<Response<IsFileStaleReply>, Status> {
    let name = request.into_inner().name;
    let file_name =
      FileName::try_from(name.clone()).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let stale = self
      .file_interactor
      .is_file_stale(&file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let reply = IsFileStaleReply { stale };
    Ok(Response::new(reply))
  }

  async fn put_file(
    &self,
    request: Request<PutFileRequest>,
  ) -> Result<Response<PutFileReply>, Status> {
    let inner = request.into_inner();
    let name = inner.name;
    let file_name =
      FileName::try_from(name.clone()).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let file_metadata = self
      .file_interactor
      .put_file(&file_name, inner.content, Some("id".to_string()))
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Failed to put file")
      })?;

    let reply = PutFileReply {
      metadata: Some(file_metadata.into()),
    };
    Ok(Response::new(reply))
  }

  async fn delete_file(
    &self,
    request: Request<proto::DeleteFileRequest>,
  ) -> Result<Response<()>, Status> {
    let name = request.into_inner().name;
    let file_name =
      FileName::try_from(name.clone()).map_err(|e| Status::invalid_argument(e.to_string()))?;
    self
      .file_interactor
      .delete_file(&file_name)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Failed to delete file")
      })?;

    Ok(Response::new(()))
  }
}
