use crate::{
  context::ApplicationContext,
  crawler::crawler::{Crawler, QueuePushParametersBuilder},
  files::file_interactor::FileInteractor,
  helpers::{key_value_store::KeyValueStore, priority::Priority},
  parser::failed_parse_files_repository::FailedParseFilesRepository,
  proto::{
    self, CrawlParseFailedFilesReply, CrawlParseFailedFilesRequest, KeyCountReply,
    MigrateSqliteRequest, ParseFileContentStoreReply,
  },
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
  crawler: Arc<Crawler>,
  file_interactor: Arc<FileInteractor>,
  failed_parse_files_repository: FailedParseFilesRepository,
  kv: Arc<KeyValueStore>,
}

impl OperationsService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      crawler: Arc::clone(&app_context.crawler),
      sqlite_connection: Arc::clone(&app_context.sqlite_connection),
      kv: Arc::clone(&app_context.kv),
      redis_connection_pool: Arc::clone(&app_context.redis_connection_pool),
      file_interactor: Arc::clone(&app_context.file_interactor),
      failed_parse_files_repository: FailedParseFilesRepository {
        redis_connection_pool: Arc::clone(&app_context.redis_connection_pool),
      },
    }
  }
}

#[tonic::async_trait]
impl proto::OperationsService for OperationsService {
  async fn get_key_value_store_size(
    &self,
    _: Request<()>,
  ) -> Result<Response<KeyCountReply>, Status> {
    let count = self.kv.size().await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Failed to get key value store size")
    })? as u32;
    let reply = KeyCountReply { count };
    Ok(Response::new(reply))
  }

  async fn delete_keys_matching(
    &self,
    request: Request<proto::KeysMatchingRequest>,
  ) -> Result<Response<()>, Status> {
    let pattern = request.into_inner().pattern;
    self.kv.delete_matching(&pattern).await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Failed to delete keys matching pattern")
    })?;
    Ok(Response::new(()))
  }

  async fn count_keys_matching(
    &self,
    request: Request<proto::KeysMatchingRequest>,
  ) -> Result<Response<proto::KeyCountReply>, Status> {
    let pattern = request.into_inner().pattern;
    let count = self.kv.count_matching(&pattern).await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Failed to count keys matching pattern")
    })? as u32;
    let reply = proto::KeyCountReply { count };
    Ok(Response::new(reply))
  }

  async fn clear_key_value_store(&self, _: Request<()>) -> Result<Response<()>, Status> {
    self.kv.clear().await.map_err(|e| {
      error!("Error: {:?}", e);
      Status::internal("Failed to clear key value store")
    })?;
    Ok(Response::new(()))
  }

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
        .crawler
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
