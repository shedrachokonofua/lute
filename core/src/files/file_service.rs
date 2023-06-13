use super::file_interactor::FileInteractor;
use crate::proto::{self, IsFileStaleReply, IsFileStaleRequest, PutFileReply, PutFileRequest};
use anyhow::Result;
use tonic::{Request, Response, Status};

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
    let stale = self
      .file_interactor
      .is_file_stale(name)
      .map_err(|e| Status::internal(e.to_string()))?;

    let reply = IsFileStaleReply { stale };
    Ok(Response::new(reply))
  }

  async fn put_file(
    &self,
    request: Request<PutFileRequest>,
  ) -> Result<Response<PutFileReply>, Status> {
    let inner = request.into_inner();
    let file_metadata = self
      .file_interactor
      .put_file(inner.name.clone(), inner.content, Some("id".to_string()))
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
