use super::{
  failed_parse_files_repository::{AggregatedError, FailedParseFilesRepository},
  parsed_file_data::{
    ParsedAlbum, ParsedAlbumSearchResult, ParsedArtist, ParsedArtistAlbum, ParsedArtistReference,
    ParsedChartAlbum, ParsedFileData, ParsedTrack,
  },
  parser::parse_file_on_store,
};
use crate::{
  events::event_publisher::EventPublisher,
  files::{
    file_content_store::FileContentStore,
    file_interactor::FileInteractor,
    file_metadata::{file_name::FileName, page_type::PageType},
  },
  helpers::fifo_queue::FifoQueue,
  proto::{
    self, EnqueueRetriesRequest, GetAggregatedFailureErrorsReply,
    GetAggregatedFailureErrorsRequest, ParseFileOnContentStoreReply,
    ParseFileOnContentStoreRequest,
  },
  settings::Settings,
};
use anyhow::{Error, Result};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;
use ulid::Ulid;

pub struct ParserService {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  failed_parse_files_repository: FailedParseFilesRepository,
  file_interactor: FileInteractor,
  settings: Arc<Settings>,
  parser_retry_queue: Arc<FifoQueue<FileName>>,
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

impl TryInto<proto::ParsedArtistReference> for ParsedArtistReference {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedArtistReference> {
    Ok(proto::ParsedArtistReference {
      name: self.name,
      file_name: self.file_name.try_into()?,
    })
  }
}

impl TryInto<proto::ParsedChartAlbum> for ParsedChartAlbum {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedChartAlbum> {
    let artists: Result<Vec<proto::ParsedArtistReference>> = self
      .artists
      .into_iter()
      .map(|artist| artist.try_into())
      .collect();

    let primary_genres = self.primary_genres;
    let secondary_genres = self.secondary_genres;
    let descriptors = self.descriptors;

    Ok(proto::ParsedChartAlbum {
      file_name: self.file_name.try_into()?,
      name: self.name,
      rating: self.rating,
      rating_count: self.rating_count,
      artists: artists?,
      primary_genres,
      secondary_genres,
      descriptors,
      release_date: self.release_date.map(|val| val.to_string()),
    })
  }
}

impl TryInto<proto::ParsedTrack> for ParsedTrack {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedTrack> {
    Ok(proto::ParsedTrack {
      name: self.name,
      duration_seconds: self.duration_seconds.try_into()?,
      rating: self.rating,
      position: self.position,
    })
  }
}

impl TryInto<proto::ParsedAlbum> for ParsedAlbum {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedAlbum> {
    let artists: Result<Vec<proto::ParsedArtistReference>> = self
      .artists
      .into_iter()
      .map(|artist| artist.try_into())
      .collect();

    let primary_genres = self.primary_genres;
    let secondary_genres = self.secondary_genres;
    let descriptors = self.descriptors;

    let tracks: Result<Vec<proto::ParsedTrack>> = self
      .tracks
      .into_iter()
      .map(|track| track.try_into())
      .collect();

    Ok(proto::ParsedAlbum {
      name: self.name,
      rating: self.rating,
      rating_count: self.rating_count,
      artists: artists?,
      primary_genres,
      secondary_genres,
      descriptors,
      tracks: tracks?,
      release_date: self.release_date.map(|val| val.to_string()),
    })
  }
}

impl TryInto<proto::ParsedArtistAlbum> for ParsedArtistAlbum {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedArtistAlbum> {
    Ok(proto::ParsedArtistAlbum {
      name: self.name,
      file_name: self.file_name.try_into()?,
    })
  }
}

impl TryInto<proto::ParsedArtist> for ParsedArtist {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedArtist> {
    let albums: Result<Vec<proto::ParsedArtistAlbum>> = self
      .albums
      .into_iter()
      .map(|album| album.try_into())
      .collect();

    Ok(proto::ParsedArtist {
      name: self.name,
      albums: albums?,
    })
  }
}

impl TryInto<proto::ParsedAlbumSearchResult> for ParsedAlbumSearchResult {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedAlbumSearchResult> {
    let artists: Result<Vec<proto::ParsedArtistReference>> = self
      .artists
      .into_iter()
      .map(|artist| artist.try_into())
      .collect();

    Ok(proto::ParsedAlbumSearchResult {
      name: self.name,
      file_name: self.file_name.try_into()?,
      artists: artists?,
    })
  }
}

impl TryInto<proto::ParsedFileData> for ParsedFileData {
  type Error = Error;

  fn try_into(self) -> Result<proto::ParsedFileData> {
    match self {
      ParsedFileData::Chart(data) => {
        let albums: Vec<proto::ParsedChartAlbum> = data
          .into_iter()
          .map(|album| album.try_into())
          .filter_map(Result::ok)
          .collect();
        let chart = proto::ParsedChart { albums };

        Ok(proto::ParsedFileData {
          data: Some(proto::parsed_file_data::Data::Chart(chart)),
        })
      }
      ParsedFileData::Album(data) => Ok(proto::ParsedFileData {
        data: Some(proto::parsed_file_data::Data::Album(data.try_into()?)),
      }),
      ParsedFileData::Artist(data) => Ok(proto::ParsedFileData {
        data: Some(proto::parsed_file_data::Data::Artist(data.try_into()?)),
      }),
      ParsedFileData::AlbumSearchResult(data) => Ok(proto::ParsedFileData {
        data: Some(proto::parsed_file_data::Data::AlbumSearchResult(
          data.try_into()?,
        )),
      }),
    }
  }
}

impl ParserService {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    parser_retry_queue: Arc<FifoQueue<FileName>>,
  ) -> Self {
    Self {
      failed_parse_files_repository: FailedParseFilesRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      file_interactor: FileInteractor::new(
        settings.file.clone(),
        Arc::clone(&redis_connection_pool),
      ),
      parser_retry_queue,
      redis_connection_pool,
      settings,
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

  async fn parse_file_on_content_store(
    &self,
    request: Request<ParseFileOnContentStoreRequest>,
  ) -> Result<Response<ParseFileOnContentStoreReply>, Status> {
    let request = request.into_inner();
    let file_name = FileName::try_from(request.file_name.clone())
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let file_metadata = self
      .file_interactor
      .get_file_metadata(&file_name)
      .await
      .map_err(|e| {
        error!(err = e.to_string(), "Failed to get file metadata");
        Status::internal("Failed to get file metadata")
      })?;
    let content_store =
      FileContentStore::new(self.settings.file.content_store.clone()).map_err(|e| {
        error!(err = e.to_string(), "Failed to create content store");
        Status::internal("Failed to create content store")
      })?;
    let parsed_data = parse_file_on_store(
      content_store,
      EventPublisher::new(Arc::clone(&self.redis_connection_pool)),
      file_metadata.id,
      file_name,
      Some(format!("rpc:{}", Ulid::new().to_string())),
    )
    .await
    .map_err(|e| {
      error!(err = e.to_string(), "Failed to parse file");
      Status::internal(format!("Failed to parse file: {}", e))
    })?;
    let result: proto::ParsedFileData = parsed_data.try_into().map_err(|e: Error| {
      error!(err = e.to_string(), "Failed to convert parsed data");
      Status::internal("Failed to convert parsed data")
    })?;
    Ok(Response::new(ParseFileOnContentStoreReply {
      data: Some(result),
    }))
  }

  async fn enqueue_retries(
    &self,
    request: Request<EnqueueRetriesRequest>,
  ) -> Result<Response<()>, Status> {
    let request: EnqueueRetriesRequest = request.into_inner();
    let error_type = request.error.to_string();
    let failures = self
      .failed_parse_files_repository
      .find_many_by_error(&error_type)
      .await
      .map_err(|err| {
        error!(err = err.to_string(), "failed to find many by error");
        Status::internal("failed to find many by error")
      })?;
    self
      .parser_retry_queue
      .push_many(failures.into_iter().map(|val| val.file_name).collect())
      .await
      .map_err(|err| {
        error!(err = err.to_string(), "failed to push many to queue");
        Status::internal("failed to push many to queue")
      })?;
    Ok(Response::new(()))
  }
}
