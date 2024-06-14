use crate::proto;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PageType {
  Artist,
  Album,
  Chart,
  AlbumSearchResult,
  ListSegment,
}

const SUPPORTED_RELEASE_TYPES: [&str; 3] = ["album", "mixtape", "ep"];

fn is_album_page(file_name: &str) -> bool {
  SUPPORTED_RELEASE_TYPES
    .iter()
    .any(|&release_type| file_name.starts_with(&format!("release/{}/", release_type)))
}

lazy_static! {
  static ref CHART_PAGE_RE: Regex = Regex::new(r"^charts/(\w+)/(album|mixtape|ep)/").unwrap();
  static ref ALBUM_SEARCH_RESULT_PAGE_RE: Regex =
    Regex::new(r"^search\?searchterm=[^&]+&searchtype=l$").unwrap();
  static ref LIST_SEGMENT_PAGE_RE: Regex = Regex::new(r"^list/(\w+)/([\w-]+)/?(\d*)/?$").unwrap();
}

fn is_chart_page(file_name: &str) -> bool {
  (*CHART_PAGE_RE).is_match(file_name)
}

fn is_list_segment_page(file_name: &str) -> bool {
  (*LIST_SEGMENT_PAGE_RE).is_match(file_name)
}

fn is_album_search_result_page(file_name: &str) -> bool {
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
      file_name if is_list_segment_page(file_name) => Ok(PageType::ListSegment),
      _ => Err(()),
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
      PageType::ListSegment => proto::PageType::ListSegmentPage,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_try_from() {
    assert_eq!(
      PageType::try_from("release/album/nas/illmatic"),
      Ok(PageType::Album)
    );
    assert_eq!(
      PageType::try_from("charts/2024/album/1/"),
      Ok(PageType::Chart)
    );
    assert_eq!(
      PageType::try_from("search?searchterm=foo&searchtype=l"),
      Ok(PageType::AlbumSearchResult)
    );
    assert_eq!(PageType::try_from("artist/foo"), Ok(PageType::Artist));
    assert_eq!(
      PageType::try_from("list/sunohara227/ethereal-sounds-of-the-internet"),
      Ok(PageType::ListSegment)
    );
    assert_eq!(
      PageType::try_from("list/sunohara227/ethereal-sounds-of-the-internet/"),
      Ok(PageType::ListSegment)
    );
    assert_eq!(
      PageType::try_from("list/sunohara227/ethereal-sounds-of-the-internet/1"),
      Ok(PageType::ListSegment)
    );
    assert_eq!(
      PageType::try_from("list/sunohara227/ethereal-sounds-of-the-internet/1/"),
      Ok(PageType::ListSegment)
    );
    assert_eq!(PageType::try_from("invalid"), Err(()));
  }
}
