use crate::proto::{Lute, LuteServer, HealthCheckReply};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Default)]
pub struct RpcServer {}

#[tonic::async_trait]
impl Lute for RpcServer {
  async fn health_check(
    &self,
    request: Request<()>
  ) -> Result<Response<HealthCheckReply>, Status> {
    println!("Got a request: {:?}", request);

    let reply = HealthCheckReply {
      ok: true,
    };

    Ok(Response::new(reply))
  }
}

pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
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