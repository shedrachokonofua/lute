use super::{AlbumSearchLookup, LookupInteractor};
use crate::{
  albums::album_read_model::{AlbumReadModel, AlbumReadModelArtist},
  context::ApplicationContext,
  files::file_metadata::file_name::ListRootFileName,
  proto,
};
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
      album_search_file_parse_error: val.album_search_file_parse_error(),
      album_search_result: val.parsed_album_search_result().map(|result| {
        proto::AlbumSearchResult {
          album_name: result.name,
          file_name: result.file_name.to_string(),
          artists: result
            .artists
            .into_iter()
            .map(|artist| AlbumReadModelArtist::from(&artist).into())
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
  lookup_interactor: Arc<LookupInteractor>,
}

impl LookupService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      lookup_interactor: Arc::clone(&app_context.lookup_interactor),
    }
  }
}

#[tonic::async_trait]
impl proto::LookupService for LookupService {
  async fn lookup_album(
    &self,
    request: Request<proto::LookupAlbumRequest>,
  ) -> Result<Response<proto::LookupAlbumReply>, Status> {
    let query = request
      .into_inner()
      .query
      .ok_or(Status::invalid_argument("query is required"))?;
    let lookup = self
      .lookup_interactor
      .search_album(query.artist_name, query.album_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::LookupAlbumReply {
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

  async fn lookup_list(
    &self,
    request: Request<proto::LookupListRequest>,
  ) -> Result<Response<proto::LookupListReply>, Status> {
    let root_file_name = ListRootFileName::try_from(request.into_inner().file_name)
      .map_err(|e| Status::invalid_argument(format!("invalid file name: {}", e.to_string())))?;
    let lookup = self
      .lookup_interactor
      .lookup_list(root_file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::LookupListReply {
      lookup: Some(lookup.into()),
    };
    Ok(Response::new(reply))
  }
}
