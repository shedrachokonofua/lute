use super::{
  parse::parse_file_on_store,
  parsed_file_data::{
    ParsedAlbum, ParsedAlbumSearchResult, ParsedArtist, ParsedArtistAlbum, ParsedArtistReference,
    ParsedChartAlbum, ParsedCredit, ParsedFileData, ParsedListSegment, ParsedTrack,
  },
  parser_failure_repository::{AggregatedError, ParserFailureRepository},
};
use crate::{
  context::ApplicationContext,
  files::file_metadata::{file_name::FileName, page_type::PageType},
  proto::{
    self, EnqueueRetriesRequest, GetAggregatedFailureErrorsReply,
    GetAggregatedFailureErrorsRequest, ParseFileOnContentStoreReply,
    ParseFileOnContentStoreRequest,
  },
  scheduler::{job_name::JobName, scheduler::JobParametersBuilder},
};
use anyhow::Result;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::error;
use ulid::Ulid;

pub struct ParserService {
  parser_failure_repository: ParserFailureRepository,
  app_context: Arc<ApplicationContext>,
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

impl From<ParsedArtistReference> for proto::ParsedArtistReference {
  fn from(val: ParsedArtistReference) -> Self {
    proto::ParsedArtistReference {
      name: val.name,
      file_name: val.file_name.to_string(),
    }
  }
}
impl Into<proto::ParsedChartAlbum> for ParsedChartAlbum {
  fn into(self) -> proto::ParsedChartAlbum {
    let artists: Vec<proto::ParsedArtistReference> = self
      .artists
      .into_iter()
      .map(|artist| artist.into())
      .collect();

    proto::ParsedChartAlbum {
      file_name: self.file_name.into(),
      name: self.name,
      rating: self.rating,
      rating_count: self.rating_count,
      artists,
      primary_genres: self.primary_genres,
      secondary_genres: self.secondary_genres,
      descriptors: self.descriptors,
      release_date: self.release_date.map(|val| val.to_string()),
    }
  }
}

impl From<ParsedTrack> for proto::ParsedTrack {
  fn from(val: ParsedTrack) -> Self {
    proto::ParsedTrack {
      name: val.name,
      duration_seconds: val.duration_seconds,
      rating: val.rating,
      position: val.position,
    }
  }
}

impl From<ParsedCredit> for proto::ParsedCredit {
  fn from(val: ParsedCredit) -> Self {
    proto::ParsedCredit {
      artist: Some(val.artist.into()),
      roles: val.roles,
    }
  }
}

impl From<ParsedAlbum> for proto::ParsedAlbum {
  fn from(val: ParsedAlbum) -> Self {
    let artists: Vec<proto::ParsedArtistReference> = val
      .artists
      .into_iter()
      .map(|artist| artist.into())
      .collect();
    let tracks: Vec<proto::ParsedTrack> =
      val.tracks.into_iter().map(|track| track.into()).collect();
    let credits: Vec<proto::ParsedCredit> = val
      .credits
      .into_iter()
      .map(|credit| credit.into())
      .collect();

    proto::ParsedAlbum {
      name: val.name,
      rating: val.rating,
      rating_count: val.rating_count,
      artists,
      primary_genres: val.primary_genres,
      secondary_genres: val.secondary_genres,
      descriptors: val.descriptors,
      tracks,
      release_date: val.release_date.map(|val| val.to_string()),
      languages: val.languages,
      credits,
      cover_image_url: val.cover_image_url,
      spotify_id: val.spotify_id,
    }
  }
}

impl From<ParsedArtistAlbum> for proto::ParsedArtistAlbum {
  fn from(val: ParsedArtistAlbum) -> Self {
    proto::ParsedArtistAlbum {
      name: val.name,
      file_name: val.file_name.into(),
    }
  }
}

impl From<ParsedArtist> for proto::ParsedArtist {
  fn from(val: ParsedArtist) -> Self {
    let albums: Vec<proto::ParsedArtistAlbum> =
      val.albums.into_iter().map(|album| album.into()).collect();

    proto::ParsedArtist {
      name: val.name,
      albums,
    }
  }
}

impl From<ParsedAlbumSearchResult> for proto::ParsedAlbumSearchResult {
  fn from(val: ParsedAlbumSearchResult) -> Self {
    let artists: Vec<proto::ParsedArtistReference> = val
      .artists
      .into_iter()
      .map(|artist| artist.into())
      .collect();

    proto::ParsedAlbumSearchResult {
      name: val.name,
      file_name: val.file_name.into(),
      artists,
    }
  }
}

impl From<ParsedListSegment> for proto::ParsedListSegment {
  fn from(val: ParsedListSegment) -> Self {
    proto::ParsedListSegment {
      name: val.name,
      page_index: val.page_index,
      other_segments: val
        .other_segments
        .into_iter()
        .map(|val| val.into())
        .collect(),
      albums: val.albums.into_iter().map(|val| val.into()).collect(),
    }
  }
}

impl From<ParsedFileData> for proto::ParsedFileData {
  fn from(val: ParsedFileData) -> Self {
    match val {
      ParsedFileData::Chart(data) => {
        let albums: Vec<proto::ParsedChartAlbum> =
          data.into_iter().map(|album| album.into()).collect();
        let chart = proto::ParsedChart { albums };

        proto::ParsedFileData {
          data: Some(proto::parsed_file_data::Data::Chart(chart)),
        }
      }
      ParsedFileData::Album(data) => proto::ParsedFileData {
        data: Some(proto::parsed_file_data::Data::Album(data.into())),
      },
      ParsedFileData::Artist(data) => proto::ParsedFileData {
        data: Some(proto::parsed_file_data::Data::Artist(data.into())),
      },
      ParsedFileData::AlbumSearchResult(data) => proto::ParsedFileData {
        data: Some(proto::parsed_file_data::Data::AlbumSearchResult(
          data.into(),
        )),
      },
      ParsedFileData::ListSegment(data) => proto::ParsedFileData {
        data: Some(proto::parsed_file_data::Data::ListSegment(data.into())),
      },
    }
  }
}

impl ParserService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      parser_failure_repository: ParserFailureRepository::new(Arc::clone(&app_context.doc_store)),
      app_context,
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
      .parser_failure_repository
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
      .app_context
      .file_interactor
      .get_file_metadata(&file_name)
      .await
      .map_err(|e| {
        error!(err = e.to_string(), "Failed to get file metadata");
        Status::internal("Failed to get file metadata")
      })?;
    let parsed_data = parse_file_on_store(
      Arc::clone(&self.app_context),
      file_metadata.id,
      file_name,
      Some(format!("rpc:{}", Ulid::new().to_string())),
    )
    .await
    .map_err(|e| {
      error!(err = e.to_string(), "Failed to parse file");
      Status::internal(format!("Failed to parse file: {}", e))
    })?;
    let result: proto::ParsedFileData = parsed_data.into();
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
      .parser_failure_repository
      .find_many(Some(error_type))
      .await
      .map_err(|err| {
        error!(err = err.to_string(), "failed to find many by error");
        Status::internal("failed to find many by error")
      })?;

    for failure in failures {
      self
        .app_context
        .scheduler
        .put(
          JobParametersBuilder::default()
            .name(JobName::ParserRetry)
            .payload(serde_json::to_vec(&failure.file_name).map_err(|err| {
              error!(err = err.to_string(), "failed to serialize payload");
              Status::internal("failed to serialize payload")
            })?)
            .build()
            .map_err(|err| {
              error!(err = err.to_string(), "failed to build job parameters");
              Status::internal("failed to build job parameters")
            })?,
        )
        .await
        .map_err(|err| {
          error!(err = err.to_string(), "failed to schedule job");
          Status::internal("failed to schedule job")
        })?;
    }
    Ok(Response::new(()))
  }
}
