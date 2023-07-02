use crate::{
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{ParsedAlbum, ParsedAlbumSearchResult},
};
use anyhow::{anyhow, Result};
use data_encoding::BASE64;
use rustis::{bb8::Pool, client::PooledClientManager, commands::StringCommands};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

pub struct AlbumSearchLookupRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum AlbumSearchLookup {
  SearchCrawling {
    query: AlbumSearchLookupQuery,
  },
  SearchParsing {
    query: AlbumSearchLookupQuery,
    album_search_file_name: FileName,
  },
  SearchParseFailed {
    query: AlbumSearchLookupQuery,
    album_search_file_name: FileName,
    album_search_file_parse_error: String,
  },
  AlbumCrawling {
    query: AlbumSearchLookupQuery,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
  },
  AlbumParsing {
    query: AlbumSearchLookupQuery,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
  },
  AlbumFileParseFailed {
    query: AlbumSearchLookupQuery,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    album_file_parse_error: String,
  },
  AlbumParsed {
    query: AlbumSearchLookupQuery,
    album_search_file_name: FileName,
    parsed_album_search_result: ParsedAlbumSearchResult,
    parsed_album: ParsedAlbum,
  },
}

impl AlbumSearchLookupRepository {}
