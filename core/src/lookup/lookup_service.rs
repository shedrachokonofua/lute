use super::{album_search_lookup::AlbumSearchLookup, lookup_interactor::LookupInteractor};
use crate::{
  albums::album_read_model_repository::{AlbumReadModel, AlbumReadModelArtist},
  proto::{self},
};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tonic::{Request, Response, Status};

impl From<AlbumSearchLookup> for proto::AlbumSearchLookup {
  fn from(val: AlbumSearchLookup) -> Self {
    proto::AlbumSearchLookup {
      query: Some(proto::AlbumSearchLookupQuery {
        artist_name: val.query().artist_name().to_string(),
        album_name: val.query().album_name().to_string(),
      }),
      last_updated_at: val.last_updated_at().map(|date| date.to_string()),
      album_search_file_name: val
        .album_search_file_name()
        .map(|file_name| file_name.to_string()),
      file_processing_correlation_id: Some(val.file_processing_correlation_id()),
      album_file_parse_error: val.album_file_parse_error(),
      album_search_file_parse_error: val
        .album_search_file_parse_error(),
      album_search_result: val.parsed_album_search_result().map(|result| {
        proto::AlbumSearchResult {
          album_name: result.name,
          file_name: result.file_name.to_string(),
          artists: result
            .artists
            .into_iter()
            .map(|artist| AlbumReadModelArtist::from_parsed_artist(&artist).into())
            .collect(),
        }
      }),
      album: val.parsed_album().map(|album| {
        AlbumReadModel::from_parsed_album(
          &val.parsed_album_search_result().unwrap().file_name,
          album,
        )
        .into()
      }),
      status: val.status_string(),
    }
  }
}

pub struct LookupService {
  lookup_interactor: LookupInteractor,
}

impl LookupService {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      lookup_interactor: LookupInteractor::new(redis_connection_pool),
    }
  }
}

#[tonic::async_trait]
impl proto::LookupService for LookupService {
  async fn search_album(
    &self,
    request: Request<proto::SearchAlbumRequest>,
  ) -> Result<Response<proto::SearchAlbumReply>, Status> {
    let query = request
      .into_inner()
      .query
      .ok_or(Status::invalid_argument("query is required"))?;
    let lookup = self
      .lookup_interactor
      .search_album(query.artist_name, query.album_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::SearchAlbumReply {
      lookup: Some(lookup.into()),
    };
    Ok(Response::new(reply))
  }

  async fn get_aggregated_album_search_statuses(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetAggregatedAlbumSearchStatusesReply>, Status> {
    let statuses = self
      .lookup_interactor
      .aggregate_statuses()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::GetAggregatedAlbumSearchStatusesReply {
      statuses: statuses
        .into_iter()
        .map(|status| proto::AggregatedStatus {
          status: status.status,
          count: status.count,
        })
        .collect(),
    };
    Ok(Response::new(reply))
  }
}
