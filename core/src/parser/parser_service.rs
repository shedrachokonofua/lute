use super::failed_parse_files_repository::{AggregatedError, FailedParseFilesRepository};
use crate::{
  files::file_metadata::page_type::PageType,
  proto::{self, GetAggregatedFailureErrorsReply, GetAggregatedFailureErrorsRequest},
};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;

pub struct ParserService {
  failed_parse_files_repository: FailedParseFilesRepository,
}

impl TryFrom<i32> for PageType {
  type Error = ();

  fn try_from(val: i32) -> Result<Self, Self::Error> {
    match val {
      0 => Ok(Self::Album),
      1 => Ok(Self::Artist),
      2 => Ok(Self::Chart),
      3 => Ok(Self::AlbumSearchResult),
      _ => Err(()),
    }
  }
}

impl ParserService {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      failed_parse_files_repository: FailedParseFilesRepository {
        redis_connection_pool,
      },
    }
  }
}

#[tonic::async_trait]
impl proto::ParserService for ParserService {
  async fn get_aggregated_failure_errors(
    &self,
    request: Request<GetAggregatedFailureErrorsRequest>,
  ) -> Result<Response<GetAggregatedFailureErrorsReply>, Status> {
    let page_type = request.into_inner().page_type.and_then(|val| {
      val.try_into().map(Some).unwrap_or_else(|_| {
        error!("invalid page type: {}", val);
        None
      })
    });
    let aggregated_errors = self
      .failed_parse_files_repository
      .aggregate_errors(page_type)
      .await
      .map_err(|err| {
        error!("failed to get aggregated errors: {:?}", err);
        Status::internal("failed to get aggregated errors")
      })?;

    let reply = GetAggregatedFailureErrorsReply {
      errors: aggregated_errors
        .into_iter()
        .map(
          |AggregatedError { error, count }| proto::AggregatedFailureError {
            error,
            count: count as u32,
          },
        )
        .collect(),
    };

    Ok(Response::new(reply))
  }
}
