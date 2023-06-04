use crate::files::{
  file_content_store::FileContentStore,
  file_metadata::{file_name::FileName, page_type::PageType},
};
use anyhow::Result;
use chrono::NaiveDate;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedChartAlbum {
  file_name: String,
  name: String,
  rating: f32,
  rating_count: u32,
  primary_genres: Vec<String>,
  secondary_genres: Vec<String>,
  descriptors: Vec<String>,
  release_date: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParsedChartEntry {
  position: u32,
  album: ParsedChartAlbum,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ParsedFileData {
  Chart { albums: Vec<ParsedChartEntry> },
}

pub async fn parse_file_on_store(
  file_content_store: FileContentStore,
  file_name: FileName,
) -> Result<ParsedFileData> {
  let file_content = file_content_store.get(&file_name).await?;
  println!("Parsing file: {}", file_name.to_string());
  match file_name.page_type() {
    PageType::Chart => {
      let albums = parse_chart(&file_content)?;
      Ok(ParsedFileData::Chart { albums })
    }
    _ => Err(anyhow::anyhow!("Unsupported page type").into()),
  }
}

pub fn parse_chart(file_content: &str) -> Result<Vec<ParsedChartEntry>> {
  let albums = Vec::new();
  Ok(albums)
}
