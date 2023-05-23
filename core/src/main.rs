use crate::rpc::run_server;

mod proto;
mod rpc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  run_server().await
}
