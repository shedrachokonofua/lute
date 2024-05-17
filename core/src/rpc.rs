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
    ParserServiceServer, ProfileServiceServer, RecommendationServiceServer, SpotifyServiceServer,
    FILE_DESCRIPTOR_SET,
  },
  recommendations::recommendation_service::RecommendationService,
  settings::Settings,
  spotify::spotify_service::SpotifyService,
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
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      settings: Arc::clone(&app_context.settings),
      file_service: Arc::new(FileService::new(Arc::clone(&app_context))),
      crawler_service: Arc::new(CrawlerService::new(Arc::clone(&app_context))),
      album_service: Arc::new(AlbumService::new(Arc::clone(&app_context))),
      spotify_service: Arc::new(SpotifyService::new(Arc::clone(&app_context))),
      operations_service: Arc::new(OperationsService::new(Arc::clone(&app_context))),
      parser_service: Arc::new(ParserService::new(Arc::clone(&app_context))),
      profile_service: Arc::new(ProfileService::new(Arc::clone(&app_context))),
      lookup_service: Arc::new(LookupService::new(Arc::clone(&app_context))),
      recommendation_service: Arc::new(RecommendationService::new(Arc::clone(&app_context))),
      event_service: Arc::new(EventService::new(Arc::clone(&app_context))),
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
