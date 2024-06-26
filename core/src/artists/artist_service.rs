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
    let (results, total) = self
      .artist_interactor
      .search(&query, request.pagination.map(Into::into).as_ref())
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    Ok(Response::new(proto::SearchArtistsReply {
      artists: results
        .into_iter()
        .map(|(artist, overview)| proto::ArtistSearchResultItem {
          artist: Some(artist.into()),
          overview: Some(overview.into()),
        })
        .collect(),
      total: total as u32,
    }))
  }

  async fn find_similar_artists(
    &self,
    request: Request<proto::FindSimilarArtistsRequest>,
  ) -> Result<Response<proto::FindSimilarArtistsReply>, Status> {
    let inner = request.into_inner();
    let file_name =
      FileName::try_from(inner.file_name).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let embedding_key = inner.embedding_key;
    let limit = inner.limit.unwrap_or(10) as usize;
    let filters: Option<ArtistSearchQuery> = inner
      .filters
      .map(|f| f.try_into())
      .transpose()
      .map_err(|e| Status::invalid_argument(format!("Invalid filters: {}", e)))?;
    let results = self
      .artist_interactor
      .find_similar_artists(file_name, &embedding_key, filters, limit)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let items = results
      .into_iter()
      .map(
        |((artist, overview), score)| proto::ArtistSimilaritySearchItem {
          artist: Some(artist.into()),
          overview: Some(overview.into()),
          score,
        },
      )
      .collect::<Vec<_>>();
    Ok(Response::new(proto::FindSimilarArtistsReply { items }))
  }
}
