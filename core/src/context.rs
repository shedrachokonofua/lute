use crate::{
  albums::{
    album_repository::AlbumRepository, album_search_index::AlbumSearchIndex,
    embedding_provider::AlbumEmbeddingProvidersInteractor,
    redis_album_search_index::RedisAlbumSearchIndex,
    sqlite_album_repository::SqliteAlbumRepository,
  },
  crawler::crawler::Crawler,
  files::{file_interactor::FileInteractor, file_metadata::file_name::FileName},
  helpers::{fifo_queue::FifoQueue, key_value_store::KeyValueStore},
  redis::build_redis_connection_pool,
  settings::Settings,
  spotify::spotify_client::SpotifyClient,
  sqlite::SqliteConnection,
  tracing::setup_tracing,
};
use anyhow::Result;
use dotenv::dotenv;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub struct ApplicationContext {
  pub settings: Arc<Settings>,
  pub sqlite_connection: Arc<SqliteConnection>,
  pub kv: Arc<KeyValueStore>,
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub parser_retry_queue: Arc<FifoQueue<FileName>>,
  pub crawler: Arc<Crawler>,
  pub album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  pub album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
  pub album_embedding_providers_interactor: Arc<AlbumEmbeddingProvidersInteractor>,
  pub spotify_client: Arc<SpotifyClient>,
  pub file_interactor: Arc<FileInteractor>,
}

impl ApplicationContext {
  pub async fn init() -> Result<Arc<ApplicationContext>> {
    dotenv().ok();
    let settings = Arc::new(Settings::new()?);
    setup_tracing(&settings.tracing)?;

    let sqlite_connection = Arc::new(SqliteConnection::new(Arc::clone(&settings)).await?);
    let kv = Arc::new(KeyValueStore::new(Arc::clone(&sqlite_connection)));
    let redis_connection_pool =
      Arc::new(build_redis_connection_pool(settings.redis.clone()).await?);
    let parser_retry_queue: Arc<FifoQueue<FileName>> = Arc::new(FifoQueue::new(
      Arc::clone(&redis_connection_pool),
      "parser:retry",
    ));
    let file_interactor = Arc::new(FileInteractor::new(
      Arc::clone(&settings),
      Arc::clone(&redis_connection_pool),
      Arc::clone(&sqlite_connection),
    ));
    let crawler = Arc::new(Crawler::new(
      Arc::clone(&settings),
      Arc::clone(&redis_connection_pool),
      Arc::clone(&file_interactor),
    )?);
    let album_repository = Arc::new(SqliteAlbumRepository::new(Arc::clone(&sqlite_connection)));
    let album_embedding_providers_interactor = Arc::new(AlbumEmbeddingProvidersInteractor::new(
      Arc::clone(&settings),
      Arc::clone(&kv),
    ));
    let album_search_index = Arc::new(RedisAlbumSearchIndex::new(
      Arc::clone(&redis_connection_pool),
      Arc::clone(&album_embedding_providers_interactor),
    ));
    let spotify_client = Arc::new(SpotifyClient::new(
      &settings.spotify.clone(),
      Arc::clone(&kv),
    ));

    Ok(Arc::new(ApplicationContext {
      settings,
      sqlite_connection,
      kv,
      redis_connection_pool,
      parser_retry_queue,
      crawler,
      album_repository,
      album_search_index,
      spotify_client,
      album_embedding_providers_interactor,
      file_interactor,
    }))
  }
}
