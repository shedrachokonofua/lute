use crate::{
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{ParsedAlbum, ParsedAlbumSearchResult},
};
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use data_encoding::BASE64;
use rustis::{bb8::Pool, client::PooledClientManager, commands::HashCommands};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub struct AlbumSearchLookupRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct AlbumSearchLookupQuery {
  pub album_name: String,
  pub artist_name: String,
}

impl AlbumSearchLookupQuery {
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

pub enum AlbumSearchLookupStatus {
  Started,
  SearchCrawling,
  SearchParsing,
  SearchParseFailed,
  AlbumCrawling,
  AlbumParsing,
  AlbumParseFailed,
  AlbumParsed,
}

impl AlbumSearchLookupStatus {
  pub fn to_string(&self) -> String {
    match self {
      AlbumSearchLookupStatus::Started => "started".to_string(),
      AlbumSearchLookupStatus::SearchCrawling => "search_crawling".to_string(),
      AlbumSearchLookupStatus::SearchParsing => "search_parsing".to_string(),
      AlbumSearchLookupStatus::SearchParseFailed => "search_parse_failed".to_string(),
      AlbumSearchLookupStatus::AlbumCrawling => "album_crawling".to_string(),
      AlbumSearchLookupStatus::AlbumParsing => "album_parsing".to_string(),
      AlbumSearchLookupStatus::AlbumParseFailed => "album_parse_failed".to_string(),
      AlbumSearchLookupStatus::AlbumParsed => "album_parsed".to_string(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "status")]
pub enum AlbumSearchLookup {
  Started {
    query: AlbumSearchLookupQuery,
  },
  SearchCrawling {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
  },
  SearchParsing {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
  },
  SearchParseFailed {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    album_search_file_parse_error: String,
  },
  AlbumCrawling {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
  },
  AlbumParsing {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
  },
  AlbumParseFailed {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    album_file_parse_error: String,
  },
  AlbumParsed {
    query: AlbumSearchLookupQuery,
    last_updated_at: NaiveDateTime,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    parsed_album: ParsedAlbum,
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
      AlbumSearchLookup::AlbumCrawling { query, .. } => query,
      AlbumSearchLookup::AlbumParsing { query, .. } => query,
      AlbumSearchLookup::AlbumParseFailed { query, .. } => query,
      AlbumSearchLookup::AlbumParsed { query, .. } => query,
    }
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
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map
      }
      AlbumSearchLookup::SearchParsing {
        query,
        album_search_file_name,
        last_updated_at,
      } => {
        let mut map = HashMap::new();
        map.insert("status".to_string(), status);
        map.insert("query".to_string(), serde_json::to_string(&query).unwrap());
        map.insert(
          "album_search_file_name".to_string(),
          album_search_file_name.to_string(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map
      }
      AlbumSearchLookup::SearchParseFailed {
        query,
        album_search_file_name,
        album_search_file_parse_error,
        last_updated_at,
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
        map
      }
      AlbumSearchLookup::AlbumCrawling {
        query,
        album_search_file_name,
        parsed_album_search_result,
        last_updated_at,
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
        map
      }
      AlbumSearchLookup::AlbumParsing {
        query,
        album_search_file_name,
        parsed_album_search_result,
        last_updated_at,
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
        map
      }
      AlbumSearchLookup::AlbumParseFailed {
        query,
        album_search_file_name,
        parsed_album_search_result,
        album_file_parse_error,
        last_updated_at,
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
        map.insert("album_file_parse_error".to_string(), album_file_parse_error);
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map
      }
      AlbumSearchLookup::AlbumParsed {
        query,
        album_search_file_name,
        parsed_album_search_result,
        parsed_album,
        last_updated_at,
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
        map.insert(
          "parsed_album".to_string(),
          serde_json::to_string(&parsed_album).unwrap(),
        );
        map.insert("last_updated_at".to_string(), last_updated_at.to_string());
        map
      }
    }
  }
}

fn get_map_field<'a>(map: &'a HashMap<String, String>, key: &'_ str) -> Result<&'a String> {
  map.get(key).ok_or(anyhow!("{} not found", key))
}

fn get_last_updated_at_field(map: &HashMap<String, String>) -> Result<NaiveDateTime> {
  get_map_field(map, "last_updated_at")?
    .parse::<NaiveDateTime>()
    .map_err(|e| anyhow!("last_updated_at parse error: {}", e))
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
      }),
      "search_parsing" => Ok(AlbumSearchLookup::SearchParsing {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
      }),
      "search_parse_failed" => Ok(AlbumSearchLookup::SearchParseFailed {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        album_search_file_parse_error: get_map_field(&value, "album_search_file_parse_error")?
          .to_string(),
      }),
      "album_crawling" => Ok(AlbumSearchLookup::AlbumCrawling {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        parsed_album_search_result: serde_json::from_str(get_map_field(
          &value,
          "parsed_album_search_result",
        )?)?,
      }),
      "album_parsing" => Ok(AlbumSearchLookup::AlbumParsing {
        query,
        album_search_file_name: get_album_search_file_name_field(&value)?,
        last_updated_at: get_last_updated_at_field(&value)?,
        parsed_album_search_result: serde_json::from_str(get_map_field(
          &value,
          "parsed_album_search_result",
        )?)?,
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
      }),
      _ => Err(anyhow!("unknown status: {}", status)),
    }
  }
}

fn key(query: &AlbumSearchLookupQuery) -> String {
  format!("lookup:album_search:{}", query.to_encoded_string())
}

fn key_from_encoded_string(encoded_string: String) -> String {
  format!("lookup:album_search:{}", encoded_string)
}

impl AlbumSearchLookupRepository {
  pub async fn find(&self, query: &AlbumSearchLookupQuery) -> Result<Option<AlbumSearchLookup>> {
    let res: HashMap<String, String> = self
      .redis_connection_pool
      .get()
      .await?
      .hgetall(key(query))
      .await?;

    match res.is_empty() {
      true => Ok(None),
      false => Ok(Some(AlbumSearchLookup::try_from(res)?)),
    }
  }

  pub async fn get(&self, query: &AlbumSearchLookupQuery) -> Result<AlbumSearchLookup> {
    match self.find(query).await? {
      Some(lookup) => Ok(lookup),
      None => Err(anyhow!("Not found")),
    }
  }

  pub async fn put(&self, lookup: &AlbumSearchLookup) -> Result<()> {
    let key = key(&lookup.query());
    let map: HashMap<String, String> = (*lookup).clone().into();
    self
      .redis_connection_pool
      .get()
      .await?
      .hset(key, map)
      .await?;
    Ok(())
  }
}
