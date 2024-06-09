use crate::proto;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum PageType {
  Artist,
  Album,
  Chart,
  AlbumSearchResult,
}

pub const SUPPORTED_RELEASE_TYPES: [&str; 3] = ["album", "mixtape", "ep"];

pub fn is_album_page(file_name: &str) -> bool {
  SUPPORTED_RELEASE_TYPES
    .iter()
    .any(|&release_type| file_name.starts_with(&format!("release/{}/", release_type)))
}

lazy_static! {
  static ref CHART_PAGE_RE: Regex = Regex::new(r"^charts/(\w+)/(album|mixtape|ep)/").unwrap();
  static ref ALBUM_SEARCH_RESULT_PAGE_RE: Regex =
    Regex::new(r"^search\?searchterm=[^&]+&searchtype=l$").unwrap();
}

fn is_chart_page(file_name: &str) -> bool {
  (*CHART_PAGE_RE).is_match(file_name)
}

pub fn is_album_search_result_page(file_name: &str) -> bool {
  (*ALBUM_SEARCH_RESULT_PAGE_RE).is_match(file_name)
}

impl TryFrom<&str> for PageType {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      file_name if is_album_page(file_name) => Ok(PageType::Album),
      file_name if is_chart_page(file_name) => Ok(PageType::Chart),
      file_name if is_album_search_result_page(file_name) => Ok(PageType::AlbumSearchResult),
      file_name if file_name.starts_with("artist") => Ok(PageType::Artist),
      _ => Err(()),
    }
  }
}

impl ToString for PageType {
  fn to_string(&self) -> String {
    match self {
      PageType::Artist => "artist".to_string(),
      PageType::Album => "album".to_string(),
      PageType::Chart => "chart".to_string(),
      PageType::AlbumSearchResult => "album_search_result".to_string(),
    }
  }
}

impl PageType {
  pub fn is_album(&self) -> bool {
    matches!(self, PageType::Album)
  }

  pub fn is_chart(&self) -> bool {
    matches!(self, PageType::Chart)
  }

  pub fn is_album_search_result(&self) -> bool {
    matches!(self, PageType::AlbumSearchResult)
  }

  pub fn is_artist(&self) -> bool {
    matches!(self, PageType::Artist)
  }
}

impl From<PageType> for proto::PageType {
  fn from(val: PageType) -> Self {
    match val {
      PageType::Artist => proto::PageType::ArtistPage,
      PageType::Album => proto::PageType::AlbumPage,
      PageType::Chart => proto::PageType::ChartPage,
      PageType::AlbumSearchResult => proto::PageType::AlbumSearchResultPage,
    }
  }
}
