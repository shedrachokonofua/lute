use crate::{
  albums::album_service::AlbumService,
  artists::artist_service::ArtistService,
  context::ApplicationContext,
  crawler::crawler_service::CrawlerService,
  events::event_service::EventService,
  files::file_service::FileService,
  lookup::LookupService,
  ops::OperationsService,
  parser::parser_service::ParserService,
  profile::profile_service::ProfileService,
  proto::{
    AlbumServiceServer, ArtistServiceServer, CrawlerServiceServer, EventServiceServer,
    FileServiceServer, HealthCheckReply, LookupServiceServer, Lute, LuteServer,
    OperationsServiceServer, ParserServiceServer, ProfileServiceServer,
    RecommendationServiceServer, SchedulerServiceServer, SpotifyServiceServer, FILE_DESCRIPTOR_SET,
  },
  recommendations::recommendation_service::RecommendationService,
  scheduler::scheduler_service::SchedulerService,
  spotify::spotify_service::SpotifyService,
};
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tokio::{task::spawn, task::JoinHandle};
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;
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
    let max_message_size = 1024 * 1024 * 1024;
    let reflection_service = tonic_reflection::server::Builder::configure()
      .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
      .build_v1()
      .unwrap();
    let addr = self.addr();
    info!(address = addr.to_string(), "Starting RPC server");
    let server = Server::builder()
      .trace_fn(|_| tracing::info_span!("lute::rpc"))
      .layer(CorsLayer::permissive())
      .layer(GrpcWebLayer::new())
      .accept_http1(true)
      .add_service(reflection_service)
      .add_service(LuteServer::new(LuteService {}))
      .add_service(FileServiceServer::new(FileService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(CrawlerServiceServer::new(CrawlerService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(
        AlbumServiceServer::new(AlbumService::new(Arc::clone(&self.app_context)))
          .max_decoding_message_size(max_message_size)
          .max_encoding_message_size(max_message_size),
      )
      .add_service(ArtistServiceServer::new(ArtistService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(SpotifyServiceServer::new(SpotifyService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(OperationsServiceServer::new(OperationsService::new(
        Arc::clone(&self.app_context),
      )))
      .add_service(ParserServiceServer::new(ParserService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(ProfileServiceServer::new(ProfileService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(LookupServiceServer::new(LookupService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(RecommendationServiceServer::new(
        RecommendationService::new(Arc::clone(&self.app_context)),
      ))
      .add_service(EventServiceServer::new(EventService::new(Arc::clone(
        &self.app_context,
      ))))
      .add_service(SchedulerServiceServer::new(SchedulerService::new(
        Arc::clone(&self.app_context),
      )));

    spawn(async move {
      if let Err(e) = server.serve(addr).await {
        eprintln!("Error running RPC server: {:?}", e);
      }
    })
  }
}
