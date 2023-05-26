use core::rpc::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  server::RpcServer::run().await
}
