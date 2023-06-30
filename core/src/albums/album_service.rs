use super::album_read_model_repository::{
  AlbumReadModel, AlbumReadModelArtist, AlbumReadModelRepository, AlbumReadModelTrack,
};
use crate::{files::file_metadata::file_name::FileName, proto};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tonic::Status;

pub struct AlbumService {
  album_read_model_repository: AlbumReadModelRepository,
}

impl From<AlbumReadModelTrack> for proto::Track {
  fn from(val: AlbumReadModelTrack) -> Self {
    proto::Track {
      name: val.name,
      duration_seconds: val.duration_seconds,
      rating: val.rating,
      position: val.position,
    }
  }
}

impl From<AlbumReadModelArtist> for proto::AlbumArtist {
  fn from(val: AlbumReadModelArtist) -> Self {
    proto::AlbumArtist {
      name: val.name,
      file_name: val.file_name.to_string(),
    }
  }
}

impl From<AlbumReadModel> for proto::Album {
  fn from(val: AlbumReadModel) -> Self {
    proto::Album {
      name: val.name,
      file_name: val.file_name.to_string(),
      rating: val.rating,
      rating_count: val.rating_count,
      artists: val
        .artists
        .into_iter()
        .map(|artist| artist.into())
        .collect(),
      primary_genres: val.primary_genres,
      secondary_genres: val.secondary_genres,
      descriptors: val.descriptors,
      tracks: val.tracks.into_iter().map(|track| track.into()).collect(),
      release_date: val.release_date.map(|date| date.to_string()),
    }
  }
}

impl AlbumService {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      album_read_model_repository: AlbumReadModelRepository {
        redis_connection_pool,
      },
    }
  }
}

#[tonic::async_trait]
impl proto::AlbumService for AlbumService {
  async fn get_album(
    &self,
    request: tonic::Request<proto::GetAlbumRequest>,
  ) -> Result<tonic::Response<proto::GetAlbumReply>, tonic::Status> {
    let file_name = FileName::try_from(request.into_inner().file_name)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let album = self
      .album_read_model_repository
      .get(&file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))
      .and_then(|album| album.ok_or_else(|| Status::not_found("Album not found")))?;
    let reply = proto::GetAlbumReply {
      album: Some(album.into()),
    };
    Ok(tonic::Response::new(reply))
  }
}
