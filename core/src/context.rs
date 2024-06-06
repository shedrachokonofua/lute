use crate::{
  albums::{
    album_interactor::AlbumInteractor, album_repository::AlbumRepository,
    album_search_index::AlbumSearchIndex, embedding_provider::AlbumEmbeddingProvidersInteractor,
    redis_album_search_index::RedisAlbumSearchIndex,
  },
  artists::artist_interactor::ArtistInteractor,
  crawler::crawler::Crawler,
  events::event_publisher::EventPublisher,
  files::file_interactor::FileInteractor,
  helpers::key_value_store::KeyValueStore,
  recommendations::spotify_track_search_index::SpotifyTrackSearchIndex,
  redis::build_redis_connection_pool,
  scheduler::scheduler::Scheduler,
  settings::Settings,
  spotify::spotify_client::SpotifyClient,
  sqlite::SqliteConnection,
  tracing::setup_tracing,
};
use anyhow::Result;
use dotenv::dotenv;
use elasticsearch::{http::transport::Transport, Elasticsearch};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub struct ApplicationContext {
  pub settings: Arc<Settings>,
  pub sqlite_connection: Arc<SqliteConnection>,
  pub kv: Arc<KeyValueStore>,
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub crawler: Arc<Crawler>,
  pub album_embedding_providers_interactor: Arc<AlbumEmbeddingProvidersInteractor>,
  pub spotify_client: Arc<SpotifyClient>,
  pub artist_interactor: Arc<ArtistInteractor>,
  pub album_interactor: Arc<AlbumInteractor>,
  pub file_interactor: Arc<FileInteractor>,
  pub event_publisher: Arc<EventPublisher>,
  pub scheduler: Arc<Scheduler>,
  pub spotify_track_search_index: Arc<SpotifyTrackSearchIndex>,
  pub elasticsearch_client: Arc<Elasticsearch>,
}

impl ApplicationContext {
  pub async fn init() -> Result<Arc<ApplicationContext>> {
    dotenv().ok();
    let settings = Arc::new(Settings::new()?);
    setup_tracing(&settings.tracing)?;

    let elasticsearch_client = Arc::new(Elasticsearch::new(Transport::single_node(
      &settings.elasticsearch.url,
    )?));
    let sqlite_connection = Arc::new(SqliteConnection::new(Arc::clone(&settings)).await?);
    let kv = Arc::new(KeyValueStore::new(Arc::clone(&sqlite_connection)));
    let redis_connection_pool =
      Arc::new(build_redis_connection_pool(settings.redis.clone()).await?);
    let event_publisher = Arc::new(EventPublisher::new(
      Arc::clone(&settings),
      Arc::clone(&sqlite_connection),
    ));
    let file_interactor = Arc::new(FileInteractor::new(
      Arc::clone(&settings),
      Arc::clone(&redis_connection_pool),
      Arc::clone(&event_publisher),
    ));
    let scheduler = Arc::new(Scheduler::new(
      Arc::clone(&sqlite_connection),
      Arc::clone(&kv),
    ));
    let crawler = Arc::new(Crawler::new(
      Arc::clone(&settings),
      Arc::clone(&scheduler),
      Arc::clone(&kv),
      Arc::clone(&file_interactor),
    )?);
    let album_repository = Arc::new(AlbumRepository::new(Arc::clone(&sqlite_connection)));
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
    let spotify_track_search_index = Arc::new(SpotifyTrackSearchIndex::new(Arc::clone(
      &redis_connection_pool,
    )));
    let album_interactor = Arc::new(AlbumInteractor::new(
      Arc::clone(&album_repository),
      Arc::clone(&album_search_index) as Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
      Arc::clone(&event_publisher),
    ));
    let artist_interactor = Arc::new(ArtistInteractor::new(
      Arc::clone(&sqlite_connection),
      Arc::clone(&elasticsearch_client),
      Arc::clone(&album_interactor),
    ));

    Ok(Arc::new(ApplicationContext {
      settings,
      sqlite_connection,
      kv,
      redis_connection_pool,
      crawler,
      spotify_client,
      album_embedding_providers_interactor,
      file_interactor,
      event_publisher,
      scheduler,
      spotify_track_search_index,
      artist_interactor,
      album_interactor,
      elasticsearch_client,
    }))
  }
}
