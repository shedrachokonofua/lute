use std::cmp::Ordering;

use super::page_type::PageType;
use anyhow::{anyhow, Result};
use serde_derive::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct FileName(String);

fn normalize_list_segment_file_name(file_name: String) -> String {
  let parts: Vec<&str> = file_name.split('/').collect();
  if parts.len() == 3 {
    return format!("{}/1", file_name);
  }
  file_name.to_string()
}

impl TryFrom<String> for FileName {
  type Error = anyhow::Error;

  fn try_from(value: String) -> Result<Self> {
    let mut clean_value = value
      .trim_start_matches('/')
      .trim_end_matches('/')
      .to_string();

    let page_type = PageType::try_from(clean_value.as_str())
      .map_err(|_| anyhow!(format!("Invalid file name: {}", value)))?;

    if page_type == PageType::ListSegment {
      clean_value = normalize_list_segment_file_name(clean_value);
    }

    Ok(Self(clean_value))
  }
}

impl TryFrom<&str> for FileName {
  type Error = anyhow::Error;

  fn try_from(value: &str) -> Result<Self> {
    Self::try_from(value.to_string())
  }
}

impl From<FileName> for String {
  fn from(val: FileName) -> Self {
    val.0
  }
}

impl ToString for FileName {
  fn to_string(&self) -> String {
    self.0.clone()
  }
}

impl PartialOrd for FileName {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for FileName {
  fn cmp(&self, other: &Self) -> Ordering {
    self.0.cmp(&other.0)
  }
}

#[derive(Default)]
pub struct ChartParameters {
  pub release_type: String,
  pub page_number: u32,
  pub years_range_start: u32,
  pub years_range_end: u32,
  pub include_primary_genres: Option<Vec<String>>,
  pub include_secondary_genres: Option<Vec<String>>,
  pub exclude_primary_genres: Option<Vec<String>>,
  pub exclude_secondary_genres: Option<Vec<String>>,
  pub include_descriptors: Option<Vec<String>>,
  pub exclude_descriptors: Option<Vec<String>>,
}

pub fn to_url_tag(value: &str) -> String {
  value.replace(' ', "-").replace('&', "and")
}

impl FileName {
  pub fn page_type(&self) -> PageType {
    PageType::try_from(self.0.as_str()).unwrap()
  }
}

impl TryInto<FileName> for ChartParameters {
  type Error = anyhow::Error;

  fn try_into(self) -> Result<FileName> {
    let mut file_name = format!(
      "charts/top/{}/{}-{}",
      self.release_type, self.years_range_start, self.years_range_end
    );

    if let Some(include_primary_genres) = self.include_primary_genres {
      file_name.push_str(
        format!(
          "/g:{}",
          include_primary_genres
            .iter()
            .map(|genre| to_url_tag(genre))
            .collect::<Vec<String>>()
            .join(",")
            .as_str()
        )
        .as_str(),
      );
    }

    if let Some(include_descriptors) = self.include_descriptors {
      file_name.push_str(
        format!(
          "/d:{}",
          include_descriptors
            .iter()
            .map(|descriptor| to_url_tag(descriptor))
            .collect::<Vec<String>>()
            .join(",")
            .as_str()
        )
        .as_str(),
      );
    }

    FileName::try_from(file_name)
  }
}
