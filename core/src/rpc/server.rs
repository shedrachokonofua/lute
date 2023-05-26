use crate::proto::{
  HealthCheckReply, IsFileStaleReply, IsFileStaleRequest, Lute, LuteServer, PutFileReply,
  PutFileRequest, ValidateFileNameReply, ValidateFileNameRequest,
};
use tonic::{transport::Server, Request, Response, Status};

use super::handlers;

#[derive(Default)]
pub struct RpcServer {}

#[tonic::async_trait]
impl Lute for RpcServer {
  async fn health_check(&self, request: Request<()>) -> Result<Response<HealthCheckReply>, Status> {
    println!("Got a request: {:?}", request);

    let reply = HealthCheckReply { ok: true };

    Ok(Response::new(reply))
  }

  async fn validate_file_name(
    &self,
    request: Request<ValidateFileNameRequest>,
  ) -> Result<Response<ValidateFileNameReply>, Status> {
    match handlers::validate_file_name(request.into_inner()) {
      Ok(reply) => Ok(Response::new(reply)),
      Err(e) => Err(Status::internal(e.to_string())),
    }
  }

  async fn is_file_stale(
    &self,
    request: Request<IsFileStaleRequest>,
  ) -> Result<Response<IsFileStaleReply>, Status> {
    println!("Got a request: {:?}", request);

    let reply = IsFileStaleReply { stale: true };

    Ok(Response::new(reply))
  }

  async fn put_file(
    &self,
    request: Request<PutFileRequest>,
  ) -> Result<Response<PutFileReply>, Status> {
    println!("Got a request: {:?}", request);

    let reply = PutFileReply { ok: true };

    Ok(Response::new(reply))
  }
}

impl RpcServer {
  pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:22000".parse().unwrap();
    let server = RpcServer::default();
  
    println!("Lute listening on {}", addr);
  
    Server::builder()
      .accept_http1(true)
      .add_service(tonic_web::enable(LuteServer::new(server)))
      .serve(addr)
      .await?;
  
    Ok(())
  }
}

