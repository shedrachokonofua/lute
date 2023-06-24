use crate::files::file_metadata::file_name::FileName;
use chrono::NaiveDate;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedArtistReference {
  pub name: String,
  pub file_name: FileName,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedChartAlbum {
  pub file_name: FileName,
  pub name: String,
  pub rating: f32,
  pub rating_count: u32,
  pub artists: Vec<ParsedArtistReference>,
  pub primary_genres: Vec<String>,
  pub secondary_genres: Vec<String>,
  pub descriptors: Vec<String>,
  pub release_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedTrack {
  pub name: String,
  pub duration_seconds: Option<u32>,
  pub rating: Option<f32>,
  pub position: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedAlbum {
  pub name: String,
  pub rating: f32,
  pub rating_count: u32,
  pub artists: Vec<ParsedArtistReference>,
  pub primary_genres: Vec<String>,
  pub secondary_genres: Vec<String>,
  pub descriptors: Vec<String>,
  pub tracks: Vec<ParsedTrack>,
  pub release_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedArtistAlbum {
  pub name: String,
  pub file_name: FileName,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedArtist {
  pub name: String,
  pub albums: Vec<ParsedArtistAlbum>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedAlbumSearchResult {
  pub name: String,
  pub file_name: FileName,
  pub artists: Vec<ParsedArtistReference>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ParsedFileData {
  Chart(Vec<ParsedChartAlbum>),
  Album(ParsedAlbum),
  Artist(ParsedArtist),
  AlbumSearchResult(ParsedAlbumSearchResult),
}
