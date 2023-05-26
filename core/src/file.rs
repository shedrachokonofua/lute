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

pub fn get_page_type(file_name: &str) -> Option<PageType> {
  match file_name {
    file_name if is_album_page(file_name) => Some(PageType::Album),
    file_name if is_chart_page(file_name) => Some(PageType::Chart),
    file_name if file_name.starts_with("search") => Some(PageType::Search),
    file_name if file_name.starts_with("artist") => Some(PageType::Artist),
    _ => None,
  }
}
