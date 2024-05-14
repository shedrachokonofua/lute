use crate::{
  albums::{
    album_repository::AlbumRepository, album_search_index::AlbumSearchIndex,
    album_service::AlbumService,
  },
  crawler::{crawler_interactor::CrawlerInteractor, crawler_service::CrawlerService},
  events::event_service::EventService,
  files::{
    file_interactor::FileInteractor, file_metadata::file_name::FileName, file_service::FileService,
  },
  helpers::{fifo_queue::FifoQueue, key_value_store::KeyValueStore},
  lookup::lookup_service::LookupService,
  ops::OperationsService,
  parser::parser_service::ParserService,
  profile::profile_service::ProfileService,
  proto::{
    AlbumServiceServer, CrawlerServiceServer, EventServiceServer, FileServiceServer,
    HealthCheckReply, LookupServiceServer, Lute, LuteServer, OperationsServiceServer,
    ParserServiceServer, ProfileServiceServer, RecommendationServiceServer, SpotifyServiceServer,
    FILE_DESCRIPTOR_SET,
  },
  recommendations::recommendation_service::RecommendationService,
  settings::Settings,
  spotify::{spotify_client::SpotifyClient, spotify_service::SpotifyService},
  sqlite::SqliteConnection,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::{net::SocketAddr, sync::Arc};
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

pub struct LuteService {}

#[tonic::async_trait]
impl Lute for LuteService {
  async fn health_check(&self, _: Request<()>) -> Result<Response<HealthCheckReply>, Status> {
    Ok(Response::new(HealthCheckReply { ok: true }))
  }
}

pub struct RpcServer {
  settings: Arc<Settings>,
  file_service: Arc<FileService>,
  crawler_service: Arc<CrawlerService>,
  album_service: Arc<AlbumService>,
  spotify_service: Arc<SpotifyService>,
  operations_service: Arc<OperationsService>,
  parser_service: Arc<ParserService>,
  profile_service: Arc<ProfileService>,
  lookup_service: Arc<LookupService>,
  recommendation_service: Arc<RecommendationService>,
  event_service: Arc<EventService>,
}

impl RpcServer {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    sqlite_connection: Arc<SqliteConnection>,
    crawler_interactor: Arc<CrawlerInteractor>,
    parser_retry_queue: Arc<FifoQueue<FileName>>,
    album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
    album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync + 'static>,
    kv: Arc<KeyValueStore>,
  ) -> Self {
    let spotify_client = Arc::new(SpotifyClient::new(
      &settings.spotify.clone(),
      Arc::clone(&kv),
    ));

    Self {
      settings: Arc::clone(&settings),
      file_service: Arc::new(FileService {
        file_interactor: FileInteractor::new(
          Arc::clone(&settings),
          Arc::clone(&redis_connection_pool),
          Arc::clone(&sqlite_connection),
        ),
      }),
      crawler_service: Arc::new(CrawlerService {
        crawler_interactor: Arc::clone(&crawler_interactor),
      }),
      album_service: Arc::new(AlbumService::new(
        Arc::clone(&settings),
        Arc::clone(&kv),
        Arc::clone(&album_repository),
        Arc::clone(&album_search_index),
        Arc::clone(&spotify_client),
      )),
      spotify_service: Arc::new(SpotifyService {
        spotify_client: Arc::clone(&spotify_client),
      }),
      operations_service: Arc::new(OperationsService::new(
        Arc::clone(&settings),
        Arc::clone(&redis_connection_pool),
        Arc::clone(&sqlite_connection),
        Arc::clone(&crawler_interactor),
      )),
      parser_service: Arc::new(ParserService::new(
        Arc::clone(&settings),
        Arc::clone(&redis_connection_pool),
        Arc::clone(&sqlite_connection),
        Arc::clone(&parser_retry_queue),
      )),
      profile_service: Arc::new(ProfileService::new(
        Arc::clone(&settings),
        Arc::clone(&redis_connection_pool),
        Arc::clone(&sqlite_connection),
        Arc::clone(&album_repository),
      )),
      lookup_service: Arc::new(LookupService::new(
        Arc::clone(&settings),
        Arc::clone(&redis_connection_pool),
        Arc::clone(&sqlite_connection),
      )),
      recommendation_service: Arc::new(RecommendationService::new(
        Arc::clone(&settings),
        Arc::clone(&redis_connection_pool),
        Arc::clone(&sqlite_connection),
        Arc::clone(&album_repository),
        Arc::clone(&album_search_index),
      )),
      event_service: Arc::new(EventService::new(Arc::clone(&sqlite_connection))),
    }
  }

  pub fn addr(&self) -> SocketAddr {
    format!("0.0.0.0:{}", &self.settings.port).parse().unwrap()
  }

  pub async fn run(&self) -> Result<()> {
    let reflection_service = tonic_reflection::server::Builder::configure()
      .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
      .build()
      .unwrap();
    let lute_service = LuteService {};
    let addr = self.addr();
    info!(address = addr.to_string(), "Starting RPC server");

    Server::builder()
      .trace_fn(|_| tracing::info_span!("core::rpc"))
      .accept_http1(true)
      .add_service(reflection_service)
      .add_service(tonic_web::enable(LuteServer::new(lute_service)))
      .add_service(tonic_web::enable(FileServiceServer::from_arc(Arc::clone(
        &self.file_service,
      ))))
      .add_service(tonic_web::enable(CrawlerServiceServer::from_arc(
        Arc::clone(&self.crawler_service),
      )))
      .add_service(tonic_web::enable(AlbumServiceServer::from_arc(Arc::clone(
        &self.album_service,
      ))))
      .add_service(tonic_web::enable(SpotifyServiceServer::from_arc(
        Arc::clone(&self.spotify_service),
      )))
      .add_service(tonic_web::enable(OperationsServiceServer::from_arc(
        Arc::clone(&self.operations_service),
      )))
      .add_service(tonic_web::enable(ParserServiceServer::from_arc(
        Arc::clone(&self.parser_service),
      )))
      .add_service(tonic_web::enable(ProfileServiceServer::from_arc(
        Arc::clone(&self.profile_service),
      )))
      .add_service(tonic_web::enable(LookupServiceServer::from_arc(
        Arc::clone(&self.lookup_service),
      )))
      .add_service(tonic_web::enable(RecommendationServiceServer::from_arc(
        Arc::clone(&self.recommendation_service),
      )))
      .add_service(tonic_web::enable(EventServiceServer::from_arc(Arc::clone(
        &self.event_service,
      ))))
      .serve(addr)
      .await?;

    Ok(())
  }
}
