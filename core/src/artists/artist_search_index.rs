use super::artist_read_model::ArtistOverview;
use crate::helpers::redisearch::get_max_num_query;
use crate::{
  files::file_metadata::file_name::FileName,
  helpers::redisearch::{
    escape_search_query_text, get_min_num_query, get_tag_query, SearchIndexVersionManager,
    SearchPagination,
  },
};
use anyhow::{anyhow, Result};
use derive_builder::Builder;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtSearchOptions, JsonCommands,
    SearchCommands, SetCondition,
  },
};
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::sync::Arc;
use tracing::instrument;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct ArtistSearchRecord {
  pub name: String,
  pub ascii_name: String,
  pub file_name: String,
  pub total_rating_count: u32,
  pub min_year: u32,
  pub max_year: u32,
  pub album_count: u32,
  pub credit_roles: Vec<String>,
  pub primary_genres: Vec<String>,
  pub creditted_primary_genres: Vec<String>,
  pub secondary_genres: Vec<String>,
  pub creditted_secondary_genres: Vec<String>,
}

impl TryFrom<Vec<(String, String)>> for ArtistSearchRecord {
  type Error = anyhow::Error;

  fn try_from(values: Vec<(String, String)>) -> Result<Self> {
    let json = values
      .first()
      .map(|(_, json)| json)
      .ok_or(anyhow!("invalid ArtistSearchRecord: missing json"))?;
    let record: ArtistSearchRecord = serde_json::from_str(json)?;
    Ok(record)
  }
}

#[derive(Debug)]
pub struct ArtistSearchResult {
  pub artists: Vec<ArtistSearchRecord>,
  pub total: usize,
}

fn get_min_year(left: u32, right: u32) -> u32 {
  if left == 0 {
    right
  } else if right == 0 {
    left
  } else {
    min(left, right)
  }
}

impl From<ArtistOverview> for ArtistSearchRecord {
  fn from(overview: ArtistOverview) -> Self {
    Self {
      name: overview.name.clone(),
      ascii_name: overview.ascii_name(),
      file_name: overview.file_name.to_string(),
      total_rating_count: overview.album_summary.total_rating_count
        + overview.credited_album_summary.total_rating_count,
      min_year: get_min_year(
        overview.album_summary.min_year,
        overview.credited_album_summary.min_year,
      ),
      max_year: max(
        overview.album_summary.max_year,
        overview.credited_album_summary.max_year,
      ),
      album_count: overview.album_summary.album_count,
      credit_roles: overview
        .credit_roles
        .into_iter()
        .map(|item| item.item)
        .collect(),
      primary_genres: overview
        .album_summary
        .primary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
      creditted_primary_genres: overview
        .credited_album_summary
        .primary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
      secondary_genres: overview
        .album_summary
        .secondary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
      creditted_secondary_genres: overview
        .credited_album_summary
        .secondary_genres
        .into_iter()
        .map(|item| item.item)
        .collect(),
    }
  }
}

#[derive(Default, Builder, Debug)]
#[builder(setter(into), default)]
pub struct ArtistSearchQuery {
  pub text: Option<String>,
  pub include_file_names: Vec<FileName>,
  pub exclude_file_names: Vec<FileName>,
  pub include_primary_genres: Vec<String>,
  pub exclude_primary_genres: Vec<String>,
  pub include_secondary_genres: Vec<String>,
  pub exclude_secondary_genres: Vec<String>,
  pub include_credit_roles: Vec<String>,
  pub exclude_credit_roles: Vec<String>,
  pub active_years_range: Option<(u32, u32)>,
  pub min_album_count: Option<u32>,
}

impl ArtistSearchQuery {
  pub fn to_ft_search_query(&self) -> String {
    let mut ft_search_query = String::from("");
    if let Some(text) = &self.text {
      ft_search_query.push_str(&format!("({}) ", escape_search_query_text(text)));
    }
    if let Some((start_year, end_year)) = self.active_years_range {
      ft_search_query.push_str(&get_min_num_query("@min_year", Some(start_year as usize)));
      ft_search_query.push_str(&get_max_num_query("@max_year", Some(end_year as usize)));
    }
    ft_search_query.push_str(&get_tag_query("@file_name", &self.include_file_names));
    ft_search_query.push_str(&get_tag_query(
      "@primary_genre",
      &self.include_primary_genres,
    ));
    ft_search_query.push_str(&get_tag_query(
      "@secondary_genre",
      &self.include_secondary_genres,
    ));
    ft_search_query.push_str(&get_tag_query("@credit_role", &self.include_credit_roles));
    ft_search_query.push_str(&get_tag_query("-@file_name", &self.exclude_file_names));
    ft_search_query.push_str(&get_tag_query(
      "-@primary_genre",
      &self.exclude_primary_genres,
    ));
    ft_search_query.push_str(&get_tag_query(
      "-@secondary_genre",
      &self.exclude_secondary_genres,
    ));
    ft_search_query.push_str(&get_tag_query("-@credit_role", &self.exclude_credit_roles));
    ft_search_query.trim().to_string()
  }
}

pub struct ArtistSearchIndex {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  version_manager: SearchIndexVersionManager,
}

const NAMESPACE: &str = "artist";
const INDEX_VERSION: u32 = 1;

fn redis_key(file_name: &FileName) -> String {
  format!("{}:{}", NAMESPACE, file_name.to_string())
}

impl ArtistSearchIndex {
  fn get_schema() -> Vec<FtFieldSchema> {
    let schema = vec![
      FtFieldSchema::identifier("$.ascii_name")
        .as_attribute("ascii_name")
        .field_type(FtFieldType::Text)
        .weight(2.0),
      FtFieldSchema::identifier("$.file_name")
        .as_attribute("file_name")
        .field_type(FtFieldType::Tag),
      FtFieldSchema::identifier("$.total_rating_count")
        .as_attribute("total_rating_count")
        .field_type(FtFieldType::Numeric)
        .sortable(),
      FtFieldSchema::identifier("$.min_year")
        .as_attribute("min_year")
        .field_type(FtFieldType::Numeric)
        .sortable(),
      FtFieldSchema::identifier("$.max_year")
        .as_attribute("max_year")
        .field_type(FtFieldType::Numeric)
        .sortable(),
      FtFieldSchema::identifier("$.album_count")
        .as_attribute("album_count")
        .field_type(FtFieldType::Numeric),
      FtFieldSchema::identifier("$.credit_roles.*")
        .as_attribute("credit_roles")
        .field_type(FtFieldType::Tag),
      FtFieldSchema::identifier("$.primary_genres.*")
        .as_attribute("primary_genres")
        .field_type(FtFieldType::Tag),
      FtFieldSchema::identifier("$.secondary_genres.*")
        .as_attribute("secondary_genres")
        .field_type(FtFieldType::Tag),
    ];
    schema
  }

  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      version_manager: SearchIndexVersionManager::new(
        Arc::clone(&redis_connection_pool),
        INDEX_VERSION,
        "artist-idx".to_string(),
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
        ArtistSearchIndex::get_schema(),
      )
      .await
  }

  fn index_name(&self) -> String {
    self.version_manager.latest_index_name()
  }

  pub async fn put(&self, artist: ArtistOverview) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    let file_name = artist.file_name.clone();
    let record = ArtistSearchRecord::from(artist);
    connection
      .json_set(
        redis_key(&file_name),
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
    query: &ArtistSearchQuery,
    pagination: Option<&SearchPagination>,
  ) -> Result<ArtistSearchResult> {
    let limit = pagination.and_then(|p| p.limit).unwrap_or(50);
    let offset = pagination.and_then(|p| p.offset).unwrap_or(0);
    let sort = pagination.and_then(|p| p.sort.clone());
    let mut options = FtSearchOptions::default().limit(offset, limit);
    if let Some((key, order)) = sort.map(|s| s.to_redisearch_sort()) {
      options = options.sortby(key, order);
    }

    let result = self
      .redis_connection_pool
      .get()
      .await?
      .ft_search(self.index_name(), query.to_ft_search_query(), options)
      .await?;

    let records = result
      .results
      .into_iter()
      .filter_map(|r| match r.values.try_into() {
        Ok(record) => Some(record),
        Err(e) => {
          tracing::warn!("Failed to deserialize ArtistSearchRecord: {}", e);
          None
        }
      })
      .collect::<Vec<_>>();

    Ok(ArtistSearchResult {
      artists: records,
      total: result.total_results,
    })
  }
}
