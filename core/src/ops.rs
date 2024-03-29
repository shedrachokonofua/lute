use crate::{
  crawler::{
    crawler_interactor::CrawlerInteractor,
    priority_queue::{Priority, QueuePushParametersBuilder},
  },
  files::file_interactor::FileInteractor,
  parser::failed_parse_files_repository::FailedParseFilesRepository,
  proto::{
    self, CrawlParseFailedFilesReply, CrawlParseFailedFilesRequest, MigrateSqliteRequest,
    ParseFileContentStoreReply,
  },
  settings::Settings,
  sqlite::SqliteConnection,
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
  sqlite_connection: Arc<SqliteConnection>,
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  crawler_interactor: Arc<CrawlerInteractor>,
  file_interactor: FileInteractor,
  failed_parse_files_repository: FailedParseFilesRepository,
}

impl OperationsService {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    sqlite_connection: Arc<SqliteConnection>,
    crawler_interactor: Arc<CrawlerInteractor>,
  ) -> Self {
    Self {
      crawler_interactor,
      sqlite_connection: Arc::clone(&sqlite_connection),
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      file_interactor: FileInteractor::new(
        settings,
        Arc::clone(&redis_connection_pool),
        sqlite_connection,
      ),
      failed_parse_files_repository: FailedParseFilesRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
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
    self
      .sqlite_connection
      .migrate_to_latest()
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Failed to migrate sqlite to latest")
      })?;
    Ok(Response::new(()))
  }

  async fn migrate_sqlite(
    &self,
    request: Request<MigrateSqliteRequest>,
  ) -> Result<Response<()>, Status> {
    let version = request.into_inner().version;
    self
      .sqlite_connection
      .migrate_to_version(version)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Failed to migrate sqlite to version")
      })?;
    Ok(Response::new(()))
  }

  async fn crawl_parse_failed_files(
    &self,
    request: Request<CrawlParseFailedFilesRequest>,
  ) -> Result<Response<CrawlParseFailedFilesReply>, Status> {
    let error = request.into_inner().error;
    let files = self
      .failed_parse_files_repository
      .find_many(error.as_deref())
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal(format!("Failed to get files by error: {}", e))
      })?
      .into_iter()
      .map(|file| file.file_name)
      .collect::<Vec<_>>();
    let count = files.len() as u32;
    for file in files {
      self
        .crawler_interactor
        .enqueue(
          QueuePushParametersBuilder::default()
            .file_name(file)
            .priority(Priority::High)
            .correlation_id("rpc:crawl_parse_failed_files")
            .build()
            .map_err(|e| {
              error!("Error: {:?}", e);
              Status::internal(format!("Failed to build queue push parameters: {}", e))
            })?,
        )
        .await
        .map_err(|e| {
          error!("Error: {:?}", e);
          Status::internal("Failed to enqueue file")
        })?;
    }
    Ok(Response::new(CrawlParseFailedFilesReply { count }))
  }
}
