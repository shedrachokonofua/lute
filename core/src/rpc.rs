use crate::{
  albums::album_service::AlbumService,
  context::ApplicationContext,
  crawler::crawler_service::CrawlerService,
  events::event_service::EventService,
  files::file_service::FileService,
  lookup::lookup_service::LookupService,
  ops::OperationsService,
  parser::parser_service::ParserService,
  profile::profile_service::ProfileService,
  proto::{
    AlbumServiceServer, CrawlerServiceServer, EventServiceServer, FileServiceServer,
    HealthCheckReply, LookupServiceServer, Lute, LuteServer, OperationsServiceServer,
    ParserServiceServer, ProfileServiceServer, RecommendationServiceServer, SchedulerServiceServer,
    SpotifyServiceServer, FILE_DESCRIPTOR_SET,
  },
  recommendations::recommendation_service::RecommendationService,
  scheduler::scheduler_service::SchedulerService,
  spotify::spotify_service::SpotifyService,
};
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tokio::{task::spawn, task::JoinHandle};
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
  app_context: Arc<ApplicationContext>,
}

impl RpcServer {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self { app_context }
  }

  pub fn addr(&self) -> SocketAddr {
    format!("0.0.0.0:{}", &self.app_context.settings.port)
      .parse()
      .unwrap()
  }

  pub fn run(&self) -> JoinHandle<()> {
    let reflection_service = tonic_reflection::server::Builder::configure()
      .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
      .build()
      .unwrap();
    let addr = self.addr();
    info!(address = addr.to_string(), "Starting RPC server");
    let server = Server::builder()
      .trace_fn(|_| tracing::info_span!("core::rpc"))
      .accept_http1(true)
      .add_service(reflection_service)
      .add_service(tonic_web::enable(LuteServer::new(LuteService {})))
      .add_service(tonic_web::enable(FileServiceServer::new(FileService::new(
        Arc::clone(&self.app_context),
      ))))
      .add_service(tonic_web::enable(CrawlerServiceServer::new(
        CrawlerService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(AlbumServiceServer::new(
        AlbumService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(SpotifyServiceServer::new(
        SpotifyService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(OperationsServiceServer::new(
        OperationsService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(ParserServiceServer::new(
        ParserService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(ProfileServiceServer::new(
        ProfileService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(LookupServiceServer::new(
        LookupService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(RecommendationServiceServer::new(
        RecommendationService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(EventServiceServer::new(
        EventService::new(Arc::clone(&self.app_context)),
      )))
      .add_service(tonic_web::enable(SchedulerServiceServer::new(
        SchedulerService::new(Arc::clone(&self.app_context)),
      )));

    spawn(async move {
      if let Err(e) = server.serve(addr).await {
        eprintln!("Error running RPC server: {:?}", e);
      }
    })
  }
}
