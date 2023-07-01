use crate::{
  albums::album_service::AlbumService,
  crawler::{crawler::Crawler, crawler_service::CrawlerService},
  files::{file_interactor::FileInteractor, file_service::FileService},
  ops::OperationsService,
  parser::parser_service::ParserService,
  profile::profile_service::ProfileService,
  proto::{
    AlbumServiceServer, CrawlerServiceServer, FileServiceServer, HealthCheckReply, Lute,
    LuteServer, OperationsServiceServer, ParserServiceServer, ProfileServiceServer,
    SpotifyServiceServer, FILE_DESCRIPTOR_SET,
  },
  settings::Settings,
  spotify::{spotify_client::SpotifyClient, spotify_service::SpotifyService},
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
  settings: Settings,
  file_service: Arc<FileService>,
  crawler_service: Arc<CrawlerService>,
  album_service: Arc<AlbumService>,
  spotify_service: Arc<SpotifyService>,
  operations_service: Arc<OperationsService>,
  parser_service: Arc<ParserService>,
  profile_service: Arc<ProfileService>,
}

impl RpcServer {
  pub fn new(
    settings: Settings,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    crawler: Arc<Crawler>,
  ) -> Self {
    Self {
      settings: settings.clone(),
      file_service: Arc::new(FileService {
        file_interactor: FileInteractor::new(
          settings.file.clone(),
          Arc::clone(&redis_connection_pool),
        ),
      }),
      crawler_service: Arc::new(CrawlerService { crawler }),
      album_service: Arc::new(AlbumService::new(Arc::clone(&redis_connection_pool))),
      spotify_service: Arc::new(SpotifyService {
        spotify_client: SpotifyClient::new(&settings.spotify, Arc::clone(&redis_connection_pool)),
      }),
      operations_service: Arc::new(OperationsService::new(
        &settings,
        Arc::clone(&redis_connection_pool),
      )),
      parser_service: Arc::new(ParserService::new(Arc::clone(&redis_connection_pool))),
      profile_service: Arc::new(ProfileService::new(Arc::clone(&redis_connection_pool))),
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
      .serve(addr)
      .await?;

    Ok(())
  }
}
