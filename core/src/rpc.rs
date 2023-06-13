use crate::{
  files::{file_interactor::FileInteractor, file_service::FileService},
  proto::{FileServiceServer, HealthCheckReply, Lute, LuteServer},
  settings::Settings,
};
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};
use tonic::{transport::Server, Request, Response, Status};

pub struct LuteService {}

#[tonic::async_trait]
impl Lute for LuteService {
  async fn health_check(&self, request: Request<()>) -> Result<Response<HealthCheckReply>, Status> {
    println!("Got a request: {:?}", request);

    let reply = HealthCheckReply { ok: true };

    Ok(Response::new(reply))
  }
}

pub struct RpcServer {
  settings: Settings,
  file_service: Arc<FileService>,
}

impl RpcServer {
  pub fn new(settings: Settings, redis_connection_pool: Arc<r2d2::Pool<redis::Client>>) -> Self {
    Self {
      settings: settings.clone(),
      file_service: Arc::new(FileService {
        file_interactor: FileInteractor::new(settings.file.clone(), redis_connection_pool.clone()),
      }),
    }
  }

  pub fn addr(&self) -> SocketAddr {
    format!("127.0.0.1:{}", &self.settings.port)
      .parse()
      .unwrap()
  }

  pub async fn run(&self) -> Result<()> {
    let lute_service = LuteService {};
    let addr = self.addr();
    println!("Starting core rpc server on {}", addr);

    Server::builder()
      .accept_http1(true)
      .add_service(tonic_web::enable(LuteServer::new(lute_service)))
      .add_service(tonic_web::enable(FileServiceServer::from_arc(
        self.file_service.clone(),
      )))
      .serve(addr)
      .await?;

    Ok(())
  }
}
