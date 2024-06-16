use crate::files::file_metadata::file_name::FileName;
use chrono::NaiveDate;
use serde_derive::{Deserialize, Serialize};
use unidecode::unidecode;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedArtistReference {
  pub name: String,
  pub file_name: FileName,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedTrack {
  pub name: String,
  pub duration_seconds: Option<u32>,
  pub rating: Option<f32>,
  pub position: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedCredit {
  pub artist: ParsedArtistReference,
  pub roles: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
  #[serde(default)]
  pub languages: Vec<String>,
  #[serde(default)]
  pub credits: Vec<ParsedCredit>,
  #[serde(default)]
  pub cover_image_url: Option<String>,
  #[serde(default)]
  pub spotify_id: Option<String>,
}

impl ParsedAlbum {
  pub fn ascii_name(&self) -> String {
    unidecode(self.name.as_str())
  }
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedArtistAlbum {
  pub name: String,
  pub file_name: FileName,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedArtist {
  pub name: String,
  pub albums: Vec<ParsedArtistAlbum>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedAlbumSearchResult {
  pub name: String,
  pub file_name: FileName,
  pub artists: Vec<ParsedArtistReference>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedListSegment {
  pub name: String,
  pub other_segments: Vec<FileName>,
  pub albums: Vec<FileName>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum ParsedFileData {
  Chart(Vec<ParsedChartAlbum>),
  Album(ParsedAlbum),
  Artist(ParsedArtist),
  AlbumSearchResult(ParsedAlbumSearchResult),
  ListSegment(ParsedListSegment),
}
