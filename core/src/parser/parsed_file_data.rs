use crate::files::file_metadata::file_name::FileName;
use chrono::NaiveDate;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedChartAlbum {
  pub file_name: FileName,
  pub name: String,
  pub rating: f32,
  pub rating_count: u32,
  pub primary_genres: Vec<String>,
  pub secondary_genres: Vec<String>,
  pub descriptors: Vec<String>,
  pub release_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ParsedFileData {
  Chart { albums: Vec<ParsedChartAlbum> },
}
