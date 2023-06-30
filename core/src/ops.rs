use crate::{
  files::file_interactor::FileInteractor,
  proto::{self, ParseFileContentStoreReply},
  settings::Settings,
};
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{FlushingMode, ServerCommands},
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;

pub struct OperationsService {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  file_interactor: FileInteractor,
}

impl OperationsService {
  pub fn new(settings: &Settings, redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      file_interactor: FileInteractor::new(settings.file.clone(), redis_connection_pool),
    }
  }
}

#[tonic::async_trait]
impl proto::OperationsService for OperationsService {
  async fn flush_redis(&self, _: Request<()>) -> Result<Response<()>, Status> {
    let connection = self.redis_connection_pool.get().await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Failed to get redis connection")
    })?;
    connection
      .flushall(FlushingMode::default())
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Failed to flush redis")
      })?;
    Ok(Response::new(()))
  }

  async fn parse_file_content_store(
    &self,
    _: Request<()>,
  ) -> Result<Response<ParseFileContentStoreReply>, Status> {
    let file_names = self.file_interactor.list_files().await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Failed to list files")
    })?;
    let count = file_names.len() as u32;
    for file_name in file_names {
      let result = self
        .file_interactor
        .put_file_metadata(&file_name, None)
        .await;
      if let Err(e) = result {
        error!("Failed to put file metadata: {:?}", e);
      }
    }

    Ok(Response::new(ParseFileContentStoreReply { count }))
  }
}
