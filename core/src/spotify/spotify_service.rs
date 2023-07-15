use super::spotify_client::{SpotifyAlbumType, SpotifyClient, SpotifyTrack};
use crate::proto::{self, HandleAuthorizationCodeRequest, IsAuthorizedReply};
use tonic::{Request, Response, Status};
use tracing::error;

pub struct SpotifyService {
  pub spotify_client: SpotifyClient,
}

impl From<SpotifyTrack> for proto::SpotifyTrack {
  fn from(track: SpotifyTrack) -> Self {
    proto::SpotifyTrack {
      spotify_id: track.spotify_id,
      name: track.name,
      artists: track
        .artists
        .iter()
        .map(|artist| proto::SpotifyArtistReference {
          spotify_id: artist.spotify_id.clone(),
          name: artist.name.clone(),
        })
        .collect(),
      album: Some(proto::SpotifyAlbumReference {
        spotify_id: track.album.spotify_id,
        name: track.album.name,
        album_type: match track.album.album_type {
          SpotifyAlbumType::Album => proto::SpotifyAlbumType::Album.into(),
          SpotifyAlbumType::Single => proto::SpotifyAlbumType::Single.into(),
          SpotifyAlbumType::Compilation => proto::SpotifyAlbumType::Compilation.into(),
        },
      }),
    }
  }
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

  async fn get_saved_tracks(
    &self,
    _: Request<()>,
  ) -> Result<Response<proto::GetSavedTracksReply>, Status> {
    let tracks = self
      .spotify_client
      .get_saved_tracks()
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?
      .into_iter()
      .map(|track| track.into())
      .collect();
    let reply = proto::GetSavedTracksReply { tracks };
    Ok(Response::new(reply))
  }

  async fn get_playlist_tracks(
    &self,
    request: Request<proto::GetPlaylistTracksRequest>,
  ) -> Result<Response<proto::GetPlaylistTracksReply>, Status> {
    let playlist_id = request.into_inner().playlist_id;
    let tracks = self
      .spotify_client
      .get_playlist_tracks(&playlist_id)
      .await
      .map_err(|e| {
        error!("Error: {:?}", e);
        Status::internal("Internal server error")
      })?
      .into_iter()
      .map(|track| track.into())
      .collect();
    let reply = proto::GetPlaylistTracksReply { tracks };
    Ok(Response::new(reply))
  }
}
