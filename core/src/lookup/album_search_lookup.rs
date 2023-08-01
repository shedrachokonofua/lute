use crate::{
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{ParsedAlbum, ParsedAlbumSearchResult},
};
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use data_encoding::BASE64;
use serde_derive::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

pub fn is_album_search_correlation_id(correlation_id: &str) -> bool {
  correlation_id.starts_with("lookup:album_search:")
}

pub fn get_album_search_correlation_id(query: &AlbumSearchLookupQuery) -> String {
  format!("lookup:album_search:{}", query.to_encoded_string())
}

pub fn get_query_from_album_search_correlation_id(
  correlation_id: &str,
) -> Result<AlbumSearchLookupQuery> {
  if !is_album_search_correlation_id(correlation_id) {
    return Err(anyhow!("Invalid album search correlation id"));
  }
  let encoded = correlation_id.replace("lookup:album_search:", "");
  AlbumSearchLookupQuery::from_encoded_string(&encoded)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub struct AlbumSearchLookupQuery {
  album_name: String,
  artist_name: String,
}

impl AlbumSearchLookupQuery {
  pub fn new(album_name: String, artist_name: String) -> Self {
    AlbumSearchLookupQuery {
      album_name: album_name.to_lowercase(),
      artist_name: artist_name.to_lowercase(),
    }
  }

  pub fn album_name(&self) -> &str {
    &self.album_name
  }

  pub fn artist_name(&self) -> &str {
    &self.artist_name
  }

  pub fn file_name(&self) -> FileName {
    let query_string = serde_urlencoded::to_string([
      (
        "searchterm",
        format!("{} {}", self.artist_name, self.album_name),
      ),
      ("searchtype", "l".to_string()),
    ])
    .expect("Failed to encode query string");
    FileName::try_from(format!("search?{}", query_string))
      .expect("Failed to create file name from query string")
  }

  pub fn to_encoded_string(&self) -> String {
    BASE64.encode(format!("{}|DELIMETER|{}", self.album_name, self.artist_name).as_bytes())
  }

  pub fn from_encoded_string(encoded: &str) -> Result<AlbumSearchLookupQuery> {
    let decoded = BASE64.decode(encoded.as_bytes())?;
    let decoded = String::from_utf8(decoded)?;
    let mut split = decoded.split("|DELIMETER|");
    let album_name = split.next().ok_or_else(|| anyhow!("No album name"))?;
    let artist_name = split.next().ok_or_else(|| anyhow!("No artist name"))?;
    Ok(AlbumSearchLookupQuery {
      album_name: album_name.to_string(),
      artist_name: artist_name.to_string(),
    })
  }
}

impl ToString for AlbumSearchLookupQuery {
  fn to_string(&self) -> String {
    self.to_encoded_string()
  }
}

#[derive(Eq, PartialEq, Serialize, Deserialize, Clone)]
pub enum AlbumSearchLookupStatus {
  Started,
  SearchCrawling,
  SearchParsing,
  SearchParseFailed,
  SearchParsed,
  AlbumCrawling,
  AlbumParsing,
  AlbumParseFailed,
  AlbumParsed,
}

pub enum AlbumSearchLookupStep {
  Started = 1,
  SearchCrawling = 2,
  SearchParsing = 3,
  SearchParseFailed = 4,
  SearchParsed = 5,
  AlbumCrawling = 6,
  AlbumParsing = 7,
  AlbumParseFailed = 8,
  AlbumParsed = 9,
}

impl AlbumSearchLookupStatus {
  pub fn to_string(&self) -> String {
    match self {
      AlbumSearchLookupStatus::Started => "started".to_string(),
      AlbumSearchLookupStatus::SearchCrawling => "search_crawling".to_string(),
      AlbumSearchLookupStatus::SearchParsing => "search_parsing".to_string(),
      AlbumSearchLookupStatus::SearchParseFailed => "search_parse_failed".to_string(),
      AlbumSearchLookupStatus::SearchParsed => "search_parsed".to_string(),
      AlbumSearchLookupStatus::AlbumCrawling => "album_crawling".to_string(),
      AlbumSearchLookupStatus::AlbumParsing => "album_parsing".to_string(),
      AlbumSearchLookupStatus::AlbumParseFailed => "album_parse_failed".to_string(),
      AlbumSearchLookupStatus::AlbumParsed => "album_parsed".to_string(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "status")]
pub enum AlbumSearchLookup {
  Started {
    query: AlbumSearchLookupQuery,
  },
  SearchCrawling {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    file_processing_correlation_id: String,
  },
  SearchParsing {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    file_processing_correlation_id: String,
  },
  SearchParseFailed {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    album_search_file_parse_error: String,
    file_processing_correlation_id: String,
  },
  SearchParsed {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    file_processing_correlation_id: String,
  },
  AlbumCrawling {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    file_processing_correlation_id: String,
  },
  AlbumParsing {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    file_processing_correlation_id: String,
  },
  AlbumParseFailed {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    album_file_parse_error: String,
    file_processing_correlation_id: String,
  },
  AlbumParsed {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    parsed_album: ParsedAlbum,
    file_processing_correlation_id: String,
  },
}

impl AlbumSearchLookup {
  pub fn new(query: AlbumSearchLookupQuery) -> Self {
    AlbumSearchLookup::Started { query }
  }

  pub fn status(&self) -> AlbumSearchLookupStatus {
    match self {
      AlbumSearchLookup::Started { .. } => AlbumSearchLookupStatus::Started,
      AlbumSearchLookup::SearchCrawling { .. } => AlbumSearchLookupStatus::SearchCrawling,
      AlbumSearchLookup::SearchParsing { .. } => AlbumSearchLookupStatus::SearchParsing,
      AlbumSearchLookup::SearchParseFailed { .. } => AlbumSearchLookupStatus::SearchParseFailed,
      AlbumSearchLookup::SearchParsed { .. } => AlbumSearchLookupStatus::SearchParsed,
      AlbumSearchLookup::AlbumCrawling { .. } => AlbumSearchLookupStatus::AlbumCrawling,
      AlbumSearchLookup::AlbumParsing { .. } => AlbumSearchLookupStatus::AlbumParsing,
      AlbumSearchLookup::AlbumParseFailed { .. } => AlbumSearchLookupStatus::AlbumParseFailed,
      AlbumSearchLookup::AlbumParsed { .. } => AlbumSearchLookupStatus::AlbumParsed,
    }
  }

  pub fn status_string(&self) -> String {
    self.status().to_string()
  }

  pub fn query(&self) -> &AlbumSearchLookupQuery {
    match self {
      AlbumSearchLookup::Started { query } => query,
      AlbumSearchLookup::SearchCrawling { query, .. } => query,
      AlbumSearchLookup::SearchParsing { query, .. } => query,
      AlbumSearchLookup::SearchParseFailed { query, .. } => query,
      AlbumSearchLookup::SearchParsed { query, .. } => query,
      AlbumSearchLookup::AlbumCrawling { query, .. } => query,
      AlbumSearchLookup::AlbumParsing { query, .. } => query,
      AlbumSearchLookup::AlbumParseFailed { query, .. } => query,
      AlbumSearchLookup::AlbumParsed { query, .. } => query,
    }
  }

  pub fn step(&self) -> u32 {
    match self {
      AlbumSearchLookup::Started { .. } => AlbumSearchLookupStep::Started as u32,
      AlbumSearchLookup::SearchCrawling { .. } => AlbumSearchLookupStep::SearchCrawling as u32,
      AlbumSearchLookup::SearchParsing { .. } => AlbumSearchLookupStep::SearchParsing as u32,
      AlbumSearchLookup::SearchParseFailed { .. } => {
        AlbumSearchLookupStep::SearchParseFailed as u32
      }
      AlbumSearchLookup::SearchParsed { .. } => AlbumSearchLookupStatus::SearchParsed as u32,
      AlbumSearchLookup::AlbumCrawling { .. } => AlbumSearchLookupStep::AlbumCrawling as u32,
      AlbumSearchLookup::AlbumParsing { .. } => AlbumSearchLookupStep::AlbumParsing as u32,
      AlbumSearchLookup::AlbumParseFailed { .. } => AlbumSearchLookupStep::AlbumParseFailed as u32,
      AlbumSearchLookup::AlbumParsed { .. } => AlbumSearchLookupStep::AlbumParsed as u32,
    }
  }

  pub fn file_processing_correlation_id(&self) -> String {
    match self {
      AlbumSearchLookup::Started { query } => get_album_search_correlation_id(query),
      AlbumSearchLookup::SearchCrawling {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
      AlbumSearchLookup::SearchParsing {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
      AlbumSearchLookup::SearchParseFailed {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
      AlbumSearchLookup::SearchParsed {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
      AlbumSearchLookup::AlbumCrawling {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
      AlbumSearchLookup::AlbumParsing {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
      AlbumSearchLookup::AlbumParseFailed {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
      AlbumSearchLookup::AlbumParsed {
        file_processing_correlation_id,
        ..
      } => file_processing_correlation_id.to_string(),
    }
  }

  pub fn parsed_album_search_result(&self) -> Option<ParsedAlbumSearchResult> {
    match self {
      AlbumSearchLookup::AlbumCrawling {
        parsed_album_search_result,
        ..
      } => Some(parsed_album_search_result.clone()),
      AlbumSearchLookup::AlbumParsing {
        parsed_album_search_result,
        ..
      } => Some(parsed_album_search_result.clone()),
      AlbumSearchLookup::AlbumParseFailed {
        parsed_album_search_result,
        ..
      } => Some(parsed_album_search_result.clone()),
      AlbumSearchLookup::AlbumParsed {
        parsed_album_search_result,
        ..
      } => Some(parsed_album_search_result.clone()),
      _ => None,
    }
  }

  pub fn parsed_album(&self) -> Option<ParsedAlbum> {
    match self {
      AlbumSearchLookup::AlbumParsed { parsed_album, .. } => Some(parsed_album.clone()),
      _ => None,
    }
  }

  pub fn last_updated_at(&self) -> Option<NaiveDateTime> {
    match self {
      AlbumSearchLookup::SearchCrawling {
        last_updated_at, ..
      } => Some(*last_updated_at),
      AlbumSearchLookup::SearchParsing {
        last_updated_at, ..
      } => Some(*last_updated_at),
      AlbumSearchLookup::SearchParseFailed {
        last_updated_at, ..
      } => Some(*last_updated_at),
      AlbumSearchLookup::SearchParsed {
        last_updated_at, ..
      } => Some(*last_updated_at),
      AlbumSearchLookup::AlbumCrawling {
        last_updated_at, ..
      } => Some(*last_updated_at),
      AlbumSearchLookup::AlbumParsing {
        last_updated_at, ..
      } => Some(*last_updated_at),
      AlbumSearchLookup::AlbumParseFailed {
        last_updated_at, ..
      } => Some(*last_updated_at),
      AlbumSearchLookup::AlbumParsed {
        last_updated_at, ..
      } => Some(*last_updated_at),
      _ => None,
    }
  }

  pub fn album_search_file_name(&self) -> Option<FileName> {
    match self {
      AlbumSearchLookup::SearchCrawling {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      AlbumSearchLookup::SearchParsing {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      AlbumSearchLookup::SearchParseFailed {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      AlbumSearchLookup::SearchParsed {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      AlbumSearchLookup::AlbumCrawling {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      AlbumSearchLookup::AlbumParsing {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      AlbumSearchLookup::AlbumParseFailed {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      AlbumSearchLookup::AlbumParsed {
        album_search_file_name,
        ..
      } => Some(album_search_file_name.clone()),
      _ => None,
    }
  }

  pub fn album_file_parse_error(&self) -> Option<String> {
    match self {
      AlbumSearchLookup::AlbumParseFailed {
        album_file_parse_error,
        ..
      } => Some(album_file_parse_error.clone()),
      _ => None,
    }
  }

  pub fn album_search_file_parse_error(&self) -> Option<String> {
    match self {
      AlbumSearchLookup::SearchParseFailed {
        album_search_file_parse_error,
        ..
      } => Some(album_search_file_parse_error.clone()),
      _ => None,
    }
  }

  pub fn can_transition(&self, target_step: AlbumSearchLookupStep, correlation_id: &str) -> bool {
    self.step() < target_step as u32 || self.file_processing_correlation_id() != correlation_id
  }
}

impl From<AlbumSearchLookup> for HashMap<String, String> {
  fn from(value: AlbumSearchLookup) -> Self {
    let status = value.status_string();
    match value {
      AlbumSearchLookup::Started { query } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map
      }
      AlbumSearchLookup::SearchCrawling {
        query,
        album_search_file_name,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
      AlbumSearchLookup::SearchParsing {
        query,
        album_search_file_name,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
      AlbumSearchLookup::SearchParseFailed {
        query,
        album_search_file_name,
        album_search_file_parse_error,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert(
          "album_search_file_parse_error".to_string(),
          album_search_file_parse_error,
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
      AlbumSearchLookup::SearchParsed {
        query,
        album_search_file_name,
        parsed_album_search_result,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert(
          "parsed_album_search_result".to_string(),
          serde_json::to_string(&parsed_album_search_result).unwrap(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
      AlbumSearchLookup::AlbumCrawling {
        query,
        album_search_file_name,
        parsed_album_search_result,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert(
          "album_file_name".to_string(),
          parsed_album_search_result.file_name.to_string(),
        );
        map.insert(
          "parsed_album_search_result".to_string(),
          serde_json::to_string(&parsed_album_search_result).unwrap(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
      AlbumSearchLookup::AlbumParsing {
        query,
        album_search_file_name,
        parsed_album_search_result,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert(
          "album_file_name".to_string(),
          parsed_album_search_result.file_name.to_string(),
        );
        map.insert(
          "parsed_album_search_result".to_string(),
          serde_json::to_string(&parsed_album_search_result).unwrap(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
      AlbumSearchLookup::AlbumParseFailed {
        query,
        album_search_file_name,
        parsed_album_search_result,
        album_file_parse_error,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert(
          "album_file_name".to_string(),
          parsed_album_search_result.file_name.to_string(),
        );
        map.insert(
          "parsed_album_search_result".to_string(),
          serde_json::to_string(&parsed_album_search_result).unwrap(),
        );
        map.insert("album_file_parse_error".to_string(), album_file_parse_error);
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
      AlbumSearchLookup::AlbumParsed {
        query,
        album_search_file_name,
        parsed_album_search_result,
        parsed_album,
        last_updated_at,
        file_processing_correlation_id,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert(
          "album_file_name".to_string(),
          parsed_album_search_result.file_name.to_string(),
        );
        map.insert(
          "parsed_album_search_result".to_string(),
          serde_json::to_string(&parsed_album_search_result).unwrap(),
        );
        map.insert(
          "parsed_album".to_string(),
          serde_json::to_string(&parsed_album).unwrap(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map.insert(
          "file_processing_correlation_id".to_string(),
          file_processing_correlation_id,
        );
        map
      }
    }
  }
}

fn get_map_field<'a>(map: &'a HashMap<String, String>, key: &'_ str) -> Result<&'a String> {
  map.get(key).ok_or(anyhow!("{} not found", key))
}

fn get_last_updated_at_field(map: &HashMap<String, String>) -> Result<NaiveDateTime> {
  NaiveDateTime::parse_from_str(
    get_map_field(map, "last_updated_at")?,
    "%Y-%m-%d %H:%M:%S%.f",
  )
  .map_err(|e| anyhow!("last_updated_at parse error: {}", e))
}

fn get_file_processing_correlation_id_field(map: &HashMap<String, String>) -> Result<String> {
  get_map_field(map, "file_processing_correlation_id").map(|s| s.to_string())
}

fn get_album_search_file_name_field(map: &HashMap<String, String>) -> Result<FileName> {
  FileName::try_from(get_map_field(map, "album_search_file_name")?.to_string())
}

impl TryFrom<HashMap<String, String>> for AlbumSearchLookup {
  type Error = anyhow::Error;

  fn try_from(value: HashMap<String, String>) -> Result<Self, Self::Error> {
    let status = get_map_field(&value, "status")?;
    let query = serde_json::from_str(get_map_field(&value, "query")?)?;

    match status.as_str() {
      "started" => Ok(AlbumSearchLookup::Started { query }),
      "search_crawling" => Ok(AlbumSearchLookup::SearchCrawling {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      "search_parsing" => Ok(AlbumSearchLookup::SearchParsing {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      "search_parse_failed" => Ok(AlbumSearchLookup::SearchParseFailed {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        album_search_file_parse_error: get_map_field(&value, "album_search_file_parse_error")?
          .to_string(),
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      "search_parsed" => Ok(AlbumSearchLookup::SearchParsed {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        parsed_album_search_result: serde_json::from_str(get_map_field(
          &value,
          "parsed_album_search_result",
        )?)?,
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      "album_crawling" => Ok(AlbumSearchLookup::AlbumCrawling {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        parsed_album_search_result: serde_json::from_str(get_map_field(
          &value,
          "parsed_album_search_result",
        )?)?,
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      "album_parsing" => Ok(AlbumSearchLookup::AlbumParsing {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        parsed_album_search_result: serde_json::from_str(get_map_field(
          &value,
          "parsed_album_search_result",
        )?)?,
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      "album_parse_failed" => Ok(AlbumSearchLookup::AlbumParseFailed {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        parsed_album_search_result: serde_json::from_str(get_map_field(
          &value,
          "parsed_album_search_result",
        )?)?,
        album_file_parse_error: get_map_field(&value, "album_file_parse_error")?.to_string(),
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      "album_parsed" => Ok(AlbumSearchLookup::AlbumParsed {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        parsed_album_search_result: serde_json::from_str(get_map_field(
          &value,
          "parsed_album_search_result",
        )?)?,
        parsed_album: serde_json::from_str(get_map_field(&value, "parsed_album")?)?,
        file_processing_correlation_id: get_file_processing_correlation_id_field(&value)?,
      }),
      _ => Err(anyhow!("unknown status: {}", status)),
    }
  }
}

impl PartialEq for AlbumSearchLookup {
  fn eq(&self, other: &Self) -> bool {
    let self_value = self.step();
    let other_value = other.step();
    self_value == other_value
  }
}

impl Eq for AlbumSearchLookup {}

impl PartialOrd for AlbumSearchLookup {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for AlbumSearchLookup {
  fn cmp(&self, other: &Self) -> Ordering {
    let self_value = self.step();
    let other_value = other.step();
    self_value.cmp(&other_value)
  }
}
