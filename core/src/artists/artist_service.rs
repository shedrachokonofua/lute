use super::{artist_interactor::ArtistInteractor, artist_search_index::ArtistSearchQuery};
use crate::{context::ApplicationContext, files::file_metadata::file_name::FileName, proto};
use async_trait::async_trait;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct ArtistService {
  artist_interactor: Arc<ArtistInteractor>,
}

impl ArtistService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      artist_interactor: Arc::clone(&app_context.artist_interactor),
    }
  }
}

#[async_trait]
impl proto::ArtistService for ArtistService {
  async fn get_artist(
    &self,
    request: Request<proto::GetArtistRequest>,
  ) -> Result<Response<proto::GetArtistReply>, Status> {
    let file_name = FileName::try_from(request.into_inner().file_name)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let artist = self
      .artist_interactor
      .find(file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(proto::GetArtistReply {
      artist: artist.map(Into::into),
    }))
  }

  async fn get_artist_overview(
    &self,
    request: Request<proto::GetArtistOverviewRequest>,
  ) -> Result<Response<proto::GetArtistOverviewReply>, Status> {
    let file_name = FileName::try_from(request.into_inner().file_name)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let overview = self
      .artist_interactor
      .get_overview(file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(proto::GetArtistOverviewReply {
      overview: overview.map(Into::into),
    }))
  }

  async fn search_artists(
    &self,
    request: Request<proto::SearchArtistsRequest>,
  ) -> Result<Response<proto::SearchArtistsReply>, Status> {
    let request = request.into_inner();
    let query = request
      .query
      .map(|q| ArtistSearchQuery::try_from(q).map_err(|e| Status::invalid_argument(e.to_string())))
      .transpose()?
      .unwrap_or_default();
    let (overviews, total) = self
      .artist_interactor
      .search(&query, request.pagination.map(Into::into).as_ref())
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(proto::SearchArtistsReply {
      overviews: overviews.into_iter().map(Into::into).collect(),
      total: total as u32,
    }))
  }
}
