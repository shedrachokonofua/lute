use super::{file_interactor::FileInteractor, file_metadata::file_name::FileName};
use crate::{
  context::ApplicationContext,
  proto::{
    self, GetFileContentReply, GetFilePageTypeReply, GetFilePageTypeRequest, IsFileStaleReply,
    IsFileStaleRequest, PutFileReply, PutFileRequest,
  },
};
use anyhow::Result;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;

pub struct FileService {
  pub file_interactor: Arc<FileInteractor>,
}

impl FileService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      file_interactor: Arc::clone(&app_context.file_interactor),
    }
  }
}

#[tonic::async_trait]
impl proto::FileService for FileService {
  async fn get_file_page_type(
    &self,
    request: Request<GetFilePageTypeRequest>,
  ) -> Result<Response<GetFilePageTypeReply>, Status> {
    Ok(Response::new(GetFilePageTypeReply {
      page_type: FileName::try_from(request.into_inner().name)
        .map(|file_name| file_name.page_type().into())
        .map(|page_type: proto::PageType| page_type.into())
        .map_err(|e| Status::invalid_argument(e.to_string()))?,
    }))
  }

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

  async fn get_file_content(
    &self,
    request: Request<proto::GetFileContentRequest>,
  ) -> Result<Response<GetFileContentReply>, Status> {
    let name = request.into_inner().name;
    let file_name =
      FileName::try_from(name.clone()).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let content = self
      .file_interactor
      .get_file_content(&file_name)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal(format!("Failed to get file content: {}", e))
      })?;

    Ok(Response::new(GetFileContentReply { content }))
  }
}
