use lazy_static::lazy_static;
use regex::Regex;

pub enum PageType {
  Artist,
  Album,
  Chart,
  Search,
}

pub const SUPPORTED_RELEASE_TYPES: [&str; 3] = ["album", "mixtape", "ep"];

fn is_album_page(file_name: &str) -> bool {
  SUPPORTED_RELEASE_TYPES
    .iter()
    .any(|&release_type| file_name.starts_with(&format!("release/{}/", release_type)))
}

lazy_static! {
  static ref CHART_PAGE_RE: Regex = Regex::new(r"^charts/(\w+)/(album|mixtape|ep)/").unwrap();
}

fn is_chart_page(file_name: &str) -> bool {
  CHART_PAGE_RE.is_match(file_name)
}

impl TryFrom<&str> for PageType {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      file_name if is_album_page(&file_name) => Ok(PageType::Album),
      file_name if is_chart_page(&file_name) => Ok(PageType::Chart),
      file_name if file_name.starts_with("search") => Ok(PageType::Search),
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
      PageType::Search => "search".to_string(),
    }
  }
}
