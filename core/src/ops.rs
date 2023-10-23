use crate::{
  files::file_interactor::FileInteractor,
  proto::{self, ParseFileContentStoreReply},
  settings::Settings,
  sqlite::{migrate_to_latest, migrate_to_version},
};

use futures::future::join_all;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{FlushingMode, ServerCommands},
};
use std::sync::Arc;
use tokio::spawn;
use tonic::{Request, Response, Status};
use tracing::error;

pub struct OperationsService {
  settings: Arc<Settings>,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  file_interactor: FileInteractor,
}

impl OperationsService {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    sqlite_connection: Arc<tokio_rusqlite::Connection>,
  ) -> Self {
    Self {
      settings: Arc::clone(&settings),
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      file_interactor: FileInteractor::new(settings, redis_connection_pool, sqlite_connection),
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

    let file_interactor = self.file_interactor.clone();
    spawn(async move {
      for chunk in file_names.chunks(20) {
        let tasks = chunk
          .iter()
          .map(|file_name| {
            let file_interactor = file_interactor.clone();
            async move {
              let result = file_interactor.put_file_metadata(file_name, None).await;
              if let Err(e) = result {
                error!("Failed to parse file content store: {:?}", e);
              }
            }
          })
          .collect::<Vec<_>>();
        join_all(tasks).await;
      }
    });
    Ok(Response::new(ParseFileContentStoreReply { count }))
  }

  async fn migrate_sqlite_to_latest(&self, _: Request<()>) -> Result<Response<()>, Status> {
    migrate_to_latest(Arc::clone(&self.settings))
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Failed to migrate sqlite to latest")
      })?;
    Ok(Response::new(()))
  }

  async fn migrate_sqlite(
    &self,
    request: Request<proto::MigrateSqliteRequest>,
  ) -> Result<Response<()>, Status> {
    let version = request.into_inner().version;
    migrate_to_version(Arc::clone(&self.settings), version)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Failed to migrate sqlite to version")
      })?;
    Ok(Response::new(()))
  }
}
