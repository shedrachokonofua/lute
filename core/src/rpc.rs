use crate::{
  albums::album_service::AlbumService,
  crawler::{crawler::Crawler, crawler_service::CrawlerService},
  files::{file_interactor::FileInteractor, file_service::FileService},
  proto::{
    AlbumServiceServer, CrawlerServiceServer, FileServiceServer, HealthCheckReply, Lute,
    LuteServer, FILE_DESCRIPTOR_SET,
  },
  settings::Settings,
};
use anyhow::Result;
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
}

impl RpcServer {
  pub fn new(
    settings: Settings,
    redis_connection_pool: Arc<r2d2::Pool<redis::Client>>,
    crawler: Arc<Crawler>,
  ) -> Self {
    Self {
      settings: settings.clone(),
      file_service: Arc::new(FileService {
        file_interactor: FileInteractor::new(settings.file, redis_connection_pool.clone()),
      }),
      crawler_service: Arc::new(CrawlerService { crawler }),
      album_service: Arc::new(AlbumService {
        redis_connection_pool: redis_connection_pool.clone(),
      }),
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
      .add_service(tonic_web::enable(FileServiceServer::from_arc(
        self.file_service.clone(),
      )))
      .add_service(tonic_web::enable(CrawlerServiceServer::from_arc(
        self.crawler_service.clone(),
      )))
      .add_service(tonic_web::enable(AlbumServiceServer::from_arc(
        self.album_service.clone(),
      )))
      .serve(addr)
      .await?;

    Ok(())
  }
}
