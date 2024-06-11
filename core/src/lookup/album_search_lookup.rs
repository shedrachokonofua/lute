use crate::{
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{ParsedAlbum, ParsedAlbumSearchResult},
};
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use data_encoding::BASE64;
use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use strum::{EnumDiscriminants, EnumString, VariantArray, VariantNames};

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

#[derive(Serialize, Deserialize, Clone, Debug, EnumDiscriminants, VariantNames)]
#[strum_discriminants(derive(strum_macros::Display, EnumString, VariantArray))]
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

  pub fn status(&self) -> AlbumSearchLookupDiscriminants {
    self.into()
  }

  pub fn status_string(&self) -> String {
    AlbumSearchLookupDiscriminants::from(self).to_string()
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
      AlbumSearchLookup::SearchParsed { .. } => AlbumSearchLookupStep::SearchParsed as u32,
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
