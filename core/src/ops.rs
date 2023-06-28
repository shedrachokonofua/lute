use crate::{
  files::file_interactor::FileInteractor,
  proto::{self, ParseFileContentStoreReply},
  settings::Settings,
};
use r2d2::Pool;
use redis::Client;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;

pub struct OperationsService {
  redis_connection_pool: Arc<Pool<Client>>,
  file_interactor: FileInteractor,
}

impl OperationsService {
  pub fn new(settings: &Settings, redis_connection_pool: Arc<Pool<Client>>) -> Self {
    Self {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      file_interactor: FileInteractor::new(settings.file.clone(), redis_connection_pool),
    }
  }
}

#[tonic::async_trait]
impl proto::OperationsService for OperationsService {
  async fn flush_redis(&self, _: Request<()>) -> Result<Response<()>, Status> {
    let mut connection = self.redis_connection_pool.get().map_err(|e| {
      println!("Error: {:?}", e);
      Status::internal("Internal server error")
    })?;
    redis::cmd("FLUSHALL")
      .query(&mut *connection)
      .map_err(|e| {
        println!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?;
    Ok(Response::new(()))
  }

  async fn parse_file_content_store(
    &self,
    _: Request<()>,
  ) -> Result<Response<ParseFileContentStoreReply>, Status> {
    let file_names = self.file_interactor.list_files().await.map_err(|e| {
      println!("Error: {:?}", e);
      Status::internal("Failed to list files")
    })?;
    let count = file_names.len() as u32;
    for file_name in file_names {
      let result = self.file_interactor.put_file_metadata(&file_name, None);
      if let Err(e) = result {
        error!("Failed to put file metadata: {:?}", e);
      }
    }

    Ok(Response::new(ParseFileContentStoreReply { count }))
  }
}
