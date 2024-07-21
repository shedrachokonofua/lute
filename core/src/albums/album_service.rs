use super::{
  album_interactor::{AlbumInteractor, AlbumMonitor},
  album_repository::{GenreAggregate, ItemAndCount},
  album_search_index::AlbumSearchQuery,
};
use crate::{
  context::ApplicationContext,
  embedding_provider::embedding_provider_interactor::EmbeddingProviderInteractor,
  files::file_metadata::file_name::FileName,
  helpers::embedding::EmbeddingDocument,
  proto,
  spotify::spotify_client::{SpotifyAlbum, SpotifyAlbumType, SpotifyClient},
};
use anyhow::{Error, Result};
use std::sync::Arc;
use tonic::{async_trait, Request, Response, Status, Streaming};

impl From<GenreAggregate> for proto::GenreAggregate {
  fn from(val: GenreAggregate) -> Self {
    proto::GenreAggregate {
      name: val.name,
      primary_genre_count: val.primary_genre_count,
      secondary_genre_count: val.secondary_genre_count,
    }
  }
}

impl From<AlbumMonitor> for proto::AlbumMonitor {
  fn from(val: AlbumMonitor) -> Self {
    proto::AlbumMonitor {
      album_count: val.album_count,
      artist_count: val.artist_count,
      genre_count: val.genre_count,
      descriptor_count: val.descriptor_count,
      duplicate_count: val.duplicate_count,
      language_count: val.language_count,
      spotify_id_count: val.spotify_id_count,
      aggregated_genres: val
        .aggregated_genres
        .into_iter()
        .map(|i| i.into())
        .collect(),
      aggregated_descriptors: val
        .aggregated_descriptors
        .into_iter()
        .map(|i| i.into())
        .collect(),
      aggregated_languages: val
        .aggregated_languages
        .into_iter()
        .map(|i| i.into())
        .collect(),
      aggregated_years: val.aggregated_years.into_iter().map(|i| i.into()).collect(),
    }
  }
}

impl From<ItemAndCount> for proto::ItemAndCount {
  fn from(val: ItemAndCount) -> Self {
    proto::ItemAndCount {
      name: val.name,
      count: val.count,
    }
  }
}

fn parse_file_name_list(file_names: Vec<String>) -> Result<Vec<FileName>> {
  file_names
    .into_iter()
    .map(|file_name| FileName::try_from(file_name).map_err(anyhow::Error::msg))
    .collect()
}

impl TryFrom<proto::AlbumSearchQuery> for AlbumSearchQuery {
  type Error = anyhow::Error;

  fn try_from(value: proto::AlbumSearchQuery) -> Result<Self> {
    Ok(AlbumSearchQuery {
      text: value.text,
      exact_name: value.exact_name,
      include_file_names: parse_file_name_list(value.include_file_names)?,
      exclude_file_names: parse_file_name_list(value.exclude_file_names)?,
      include_artists: parse_file_name_list(value.include_artists)?,
      exclude_artists: parse_file_name_list(value.exclude_artists)?,
      include_primary_genres: value.include_primary_genres,
      exclude_primary_genres: value.exclude_primary_genres,
      include_secondary_genres: value.include_secondary_genres,
      exclude_secondary_genres: value.exclude_secondary_genres,
      include_languages: value.include_languages,
      exclude_languages: value.exclude_languages,
      include_descriptors: value.include_descriptors,
      exclude_descriptors: value.exclude_descriptors,
      min_primary_genre_count: value.min_primary_genre_count.map(|i| i as usize),
      min_secondary_genre_count: value.min_secondary_genre_count.map(|i| i as usize),
      min_descriptor_count: value.min_descriptor_count.map(|i| i as usize),
      min_release_year: value.min_release_year,
      max_release_year: value.max_release_year,
      include_duplicates: value.include_duplicates,
    })
  }
}

impl TryFrom<SpotifyAlbum> for proto::SpotifyAlbum {
  type Error = anyhow::Error;

  fn try_from(value: SpotifyAlbum) -> Result<Self> {
    Ok(proto::SpotifyAlbum {
      spotify_id: value.spotify_id,
      name: value.name,
      artists: value
        .artists
        .into_iter()
        .map(|artist| proto::SpotifyArtistReference {
          spotify_id: artist.spotify_id,
          name: artist.name,
        })
        .collect(),
      album_type: match value.album_type {
        SpotifyAlbumType::Album => proto::SpotifyAlbumType::Album.into(),
        SpotifyAlbumType::Single => proto::SpotifyAlbumType::Single.into(),
        SpotifyAlbumType::Compilation => proto::SpotifyAlbumType::Compilation.into(),
        SpotifyAlbumType::AppearsOn => proto::SpotifyAlbumType::ApprearsOn.into(),
      },
      tracks: value.tracks.into_iter().map(|track| track.into()).collect(),
    })
  }
}
pub struct AlbumService {
  embedding_provider_interactor: Arc<EmbeddingProviderInteractor>,
  album_interactor: Arc<AlbumInteractor>,
  spotify_client: Arc<SpotifyClient>,
}

impl AlbumService {
  pub fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      embedding_provider_interactor: Arc::clone(&app_context.embedding_provider_interactor),
      album_interactor: Arc::clone(&app_context.album_interactor),
      spotify_client: Arc::clone(&app_context.spotify_client),
    }
  }
}

#[async_trait]
impl proto::AlbumService for AlbumService {
  async fn get_monitor(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetAlbumMonitorReply>, Status> {
    let monitor = self
      .album_interactor
      .get_monitor()
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::GetAlbumMonitorReply {
      monitor: Some(monitor.into()),
    };
    Ok(Response::new(reply))
  }

  async fn get_album(
    &self,
    request: Request<proto::GetAlbumRequest>,
  ) -> Result<Response<proto::GetAlbumReply>, Status> {
    let file_name = FileName::try_from(request.into_inner().file_name)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let album = self
      .album_interactor
      .get(&file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::GetAlbumReply {
      album: Some(album.into()),
    };
    Ok(Response::new(reply))
  }

  async fn get_many_albums(
    &self,
    request: Request<proto::GetManyAlbumsRequest>,
  ) -> Result<Response<proto::GetManyAlbumsReply>, Status> {
    let file_names = parse_file_name_list(request.into_inner().file_names)
      .map_err(|e| Status::invalid_argument(e.to_string()))?;
    let albums = self
      .album_interactor
      .get_many(file_names)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::GetManyAlbumsReply {
      albums: albums.into_iter().map(|album| album.into()).collect(),
    };
    Ok(Response::new(reply))
  }

  async fn search_albums(
    &self,
    request: Request<proto::SearchAlbumsRequest>,
  ) -> Result<Response<proto::SearchAlbumsReply>, Status> {
    let request = request.into_inner();
    let query: AlbumSearchQuery = request
      .query
      .map(|q| q.try_into())
      .transpose()
      .map_err(|e: Error| Status::invalid_argument(format!("Invalid query: {}", e)))?
      .unwrap_or_default();
    let pagination = request.pagination.map(|p| p.into());
    let results = self
      .album_interactor
      .search(&query, pagination.as_ref())
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::SearchAlbumsReply {
      albums: results
        .albums
        .into_iter()
        .map(|album| album.into())
        .collect::<Vec<proto::Album>>(),
      total: results.total as u32,
    };
    Ok(Response::new(reply))
  }

  async fn get_embedding_keys(
    &self,
    _request: Request<()>,
  ) -> Result<Response<proto::GetEmbeddingKeysReply>, Status> {
    let reply = proto::GetEmbeddingKeysReply {
      keys: self
        .embedding_provider_interactor
        .providers
        .keys()
        .map(|provider| provider.to_string())
        .collect(),
    };
    Ok(Response::new(reply))
  }

  async fn find_similar_albums(
    &self,
    request: Request<proto::FindSimilarAlbumsRequest>,
  ) -> Result<Response<proto::FindSimilarAlbumsReply>, Status> {
    let inner = request.into_inner();
    let file_name =
      FileName::try_from(inner.file_name).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let embedding_key = inner.embedding_key;
    let limit = inner.limit.unwrap_or(10) as usize;
    let filters: Option<AlbumSearchQuery> = inner
      .filters
      .map(|f| f.try_into())
      .transpose()
      .map_err(|e| Status::invalid_argument(format!("Invalid filters: {}", e)))?;
    let results = self
      .album_interactor
      .find_similar_albums(file_name, &embedding_key, filters, limit)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::FindSimilarAlbumsReply {
      albums: results.into_iter().map(Into::into).collect(),
    };
    Ok(Response::new(reply))
  }

  async fn find_spotify_album(
    &self,
    request: Request<proto::FindSpotifyAlbumRequest>,
  ) -> Result<Response<proto::FindSpotifyAlbumReply>, Status> {
    let request = request.into_inner();
    let file_name =
      FileName::try_from(request.file_name).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let album = self
      .album_interactor
      .get(&file_name)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let spotify_album = self
      .spotify_client
      .find_album(&album)
      .await
      .map_err(|e| Status::internal(e.to_string()))?;
    let reply = proto::FindSpotifyAlbumReply {
      album: spotify_album.map(|a| a.try_into().unwrap()),
    };
    Ok(Response::new(reply))
  }

  async fn bulk_upload_album_embeddings(
    &self,
    request: Request<Streaming<proto::BulkUploadAlbumEmbeddingsRequest>>,
  ) -> Result<Response<proto::BulkUploadAlbumEmbeddingsReply>, Status> {
    let mut input_stream = request.into_inner();
    let mut count: u32 = 0;
    while let Some(upload_request) = input_stream.message().await? {
      count += upload_request.items.len() as u32;
      self
        .album_interactor
        .put_many_embeddings(
          upload_request
            .items
            .into_iter()
            .filter_map(|item| {
              let file_name = FileName::try_from(item.file_name).ok()?;
              Some(EmbeddingDocument {
                file_name,
                key: item.embedding_key,
                embedding: item.embedding,
              })
            })
            .collect(),
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
    }
    Ok(Response::new(proto::BulkUploadAlbumEmbeddingsReply {
      count,
    }))
  }
}
