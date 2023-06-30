use super::spotify_client::SpotifyClient;
use crate::proto::{self, HandleAuthorizationCodeRequest, IsAuthorizedReply};
use tonic::{Request, Response, Status};
use tracing::error;

pub struct SpotifyService {
  pub spotify_client: SpotifyClient,
}

#[tonic::async_trait]
impl proto::SpotifyService for SpotifyService {
  async fn is_authorized(&self, _: Request<()>) -> Result<Response<IsAuthorizedReply>, Status> {
    let reply = IsAuthorizedReply {
      authorized: self.spotify_client.is_authorized().await,
    };
    Ok(Response::new(reply))
  }

  async fn get_authorization_url(
    &self,
    _: Request<()>,
  ) -> Result<Response<proto::GetAuthorizationUrlReply>, Status> {
    let reply = proto::GetAuthorizationUrlReply {
      url: self.spotify_client.get_authorize_url().map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?,
    };
    Ok(Response::new(reply))
  }

  async fn handle_authorization_code(
    &self,
    request: Request<HandleAuthorizationCodeRequest>,
  ) -> std::result::Result<Response<()>, Status> {
    self
      .spotify_client
      .receive_auth_code(&request.into_inner().code)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?;

    Ok(Response::new(()))
  }
}
