use crate::{
  albums::album_search_index::embedding_to_bytes,
  files::file_metadata::file_name::FileName,
  helpers::redisearch::{get_tag_query, SearchIndexVersionManager, SearchPagination},
  spotify::spotify_client::{SpotifyAlbumReference, SpotifyArtistReference, SpotifyTrackReference},
};
use anyhow::{anyhow, Result};
use derive_builder::Builder;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtCreateOptions, FtFieldSchema, FtFieldType, FtFlatVectorFieldAttributes, FtIndexDataType,
    FtSearchOptions, FtVectorDistanceMetric, FtVectorFieldAlgorithm, FtVectorType, JsonCommands,
    SearchCommands, SetCondition, SortOrder,
  },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{instrument, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotifyTrackSearchRecord {
  pub spotify_id: String,
  pub name: String,
  pub album_file_name: FileName,
  pub album: SpotifyAlbumReference,
  pub artists: Vec<SpotifyArtistReference>,
  pub embedding: Vec<f32>,
}

impl Into<SpotifyTrackReference> for SpotifyTrackSearchRecord {
  fn into(self) -> SpotifyTrackReference {
    SpotifyTrackReference {
      spotify_id: self.spotify_id,
      name: self.name,
      artists: self.artists,
    }
  }
}

impl SpotifyTrackSearchRecord {
  pub fn new(
    track: SpotifyTrackReference,
    album: SpotifyAlbumReference,
    album_file_name: FileName,
    embedding: Vec<f32>,
  ) -> Self {
    Self {
      spotify_id: track.spotify_id,
      name: track.name,
      album_file_name,
      album,
      artists: track.artists,
      embedding,
    }
  }
}

impl TryFrom<Vec<(String, String)>> for SpotifyTrackSearchRecord {
  type Error = anyhow::Error;

  fn try_from(values: Vec<(String, String)>) -> Result<Self> {
    let json = values
      .get(0)
      .map(|(_, json)| json)
      .ok_or(anyhow!("invalid SpotifyTrackSearchRecord: missing json"))?;
    let record: SpotifyTrackSearchRecord = serde_json::from_str(json)?;
    Ok(record)
  }
}

#[derive(Debug)]
pub struct SpotifyTrackSearchResult {
  pub tracks: Vec<SpotifyTrackSearchRecord>,
  pub total: usize,
}

#[derive(Default, Builder, Debug)]
#[builder(setter(into), default)]
pub struct SpotifyTrackQuery {
  pub include_spotify_ids: Vec<String>,
  pub include_album_file_names: Vec<FileName>,
}

impl SpotifyTrackQuery {
  pub fn to_ft_search_query(&self) -> String {
    let mut query = String::from("");
    query.push_str(&get_tag_query("@spotify_id", &self.include_spotify_ids));
    query.push_str(&get_tag_query(
      "@album_file_name",
      &self.include_album_file_names,
    ));
    query.trim().to_string()
  }
}

#[derive(Debug)]
pub struct SpotifyTrackEmbeddingSimilaritySearchQuery {
  pub embedding: Vec<f32>,
  pub filters: SpotifyTrackQuery,
  pub limit: usize,
}

impl SpotifyTrackEmbeddingSimilaritySearchQuery {
  pub fn to_ft_search_query(&self) -> String {
    format!(
      "({})=>[KNN {} @embedding $BLOB as distance]",
      self.filters.to_ft_search_query(),
      self.limit,
    )
  }
}

pub struct SpotifyTrackSimilairtySearchQuery {
  pub embedding: Vec<f32>,
  pub limit: usize,
  pub filters: SpotifyTrackQuery,
}

pub struct SpotifyTrackSearchIndex {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  version_manager: SearchIndexVersionManager,
}

const NAMESPACE: &str = "spotify_track";
const INDEX_VERSION: u32 = 1;

impl SpotifyTrackSearchIndex {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      version_manager: SearchIndexVersionManager::new(
        Arc::clone(&redis_connection_pool),
        INDEX_VERSION,
        "spotify_track_idx".to_string(),
      ),
      redis_connection_pool,
    }
  }

  pub async fn setup_index(&self) -> Result<()> {
    self
      .version_manager
      .setup_index(
        FtCreateOptions::default()
          .on(FtIndexDataType::Json)
          .prefix(format!("{}:", NAMESPACE)),
        vec![
          FtFieldSchema::identifier("$.spotify_id")
            .as_attribute("spotify_id")
            .field_type(FtFieldType::Tag),
          FtFieldSchema::identifier("$.name")
            .as_attribute("name")
            .field_type(FtFieldType::Text),
          FtFieldSchema::identifier("$.album_file_name")
            .as_attribute("album_file_name")
            .field_type(FtFieldType::Tag),
          FtFieldSchema::identifier("$.album.spotify_id")
            .as_attribute("album_spotify_id")
            .field_type(FtFieldType::Tag),
          FtFieldSchema::identifier("$.album.name")
            .as_attribute("album_name")
            .field_type(FtFieldType::Text),
          FtFieldSchema::identifier("$.artists[*].spotify_id")
            .as_attribute("artist_spotify_id")
            .field_type(FtFieldType::Tag),
          FtFieldSchema::identifier("$.artists[*].name")
            .as_attribute("artist_name")
            .field_type(FtFieldType::Text),
          FtFieldSchema::identifier("$.embedding")
            .as_attribute("embedding")
            .field_type(FtFieldType::Vector(Some(FtVectorFieldAlgorithm::Flat(
              FtFlatVectorFieldAttributes::new(
                FtVectorType::Float32,
                9,
                FtVectorDistanceMetric::Cosine,
              ),
            )))),
        ],
      )
      .await
  }

  pub async fn put(&self, record: SpotifyTrackSearchRecord) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        format!("{}:{}", NAMESPACE, record.spotify_id),
        "$",
        serde_json::to_string(&record)?,
        SetCondition::default(),
      )
      .await?;

    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn search(
    &self,
    query: &SpotifyTrackQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<SpotifyTrackSearchResult> {
    let limit = pagination.and_then(|p| p.limit).unwrap_or(100000);
    let offset = pagination.and_then(|p| p.offset).unwrap_or(0);
    let result = self
      .redis_connection_pool
      .get()
      .await?
      .ft_search(
        self.version_manager.latest_index_name(),
        query.to_ft_search_query(),
        FtSearchOptions::default().limit(offset, limit),
      )
      .await?;

    let tracks = result
      .results
      .into_iter()
      .filter_map(|r| match r.values.try_into() {
        Ok(track) => Some(track),
        Err(e) => {
          warn!("Failed to deserialize SpotifyTrackSearchRecord: {}", e);
          None
        }
      })
      .collect::<Vec<_>>();

    Ok(SpotifyTrackSearchResult {
      tracks,
      total: result.total_results,
    })
  }

  #[instrument(skip(self))]
  pub async fn embedding_similarity_search(
    &self,
    query: &SpotifyTrackEmbeddingSimilaritySearchQuery,
  ) -> Result<Vec<(SpotifyTrackSearchRecord, f32)>> {
    let search_result = self
      .redis_connection_pool
      .get()
      .await?
      .ft_search(
        self.version_manager.latest_index_name(),
        query.to_ft_search_query(),
        FtSearchOptions::default()
          .params(("BLOB", embedding_to_bytes(&query.embedding)))
          .dialect(2)
          .limit(0, query.limit)
          .sortby("distance", SortOrder::Asc),
      )
      .await?;
    let results = search_result
      .results
      .into_iter()
      .filter_map(|row| {
        let distance = row
          .values
          .get(0)
          .map(|(_, distance)| distance.parse::<f32>().ok())??;
        let track = row
          .values
          .get(1)
          .and_then(|(_, json)| serde_json::from_str::<SpotifyTrackSearchRecord>(json).ok())?;
        Some((track, distance))
      })
      .collect::<Vec<_>>();
    Ok(results)
  }
}
