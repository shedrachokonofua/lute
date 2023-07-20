use super::page_type::PageType;
use anyhow::Result;
use serde_derive::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct FileName(pub String);

impl TryFrom<String> for FileName {
  type Error = anyhow::Error;

  fn try_from(value: String) -> Result<Self> {
    let clean_value = value
      .trim_start_matches('/')
      .trim_end_matches('/')
      //.replace("’", "'")
      .to_string();
    match PageType::try_from(clean_value.as_str()) {
      Ok(_) => Ok(Self(clean_value)),
      Err(_) => Err(anyhow::Error::msg(format!("Invalid file name: {}", value))),
    }
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
  value.replace(" ", "-").replace("&", "and")
}

impl FileName {
  pub fn page_type(&self) -> PageType {
    PageType::try_from(self.0.as_str()).unwrap()
  }

  pub fn create_chart_file_name(params: ChartParameters) -> Result<FileName> {
    let mut file_name = format!(
      "charts/top/{}/{}-{}",
      params.release_type, params.years_range_start, params.years_range_end
    );

    if let Some(include_primary_genres) = params.include_primary_genres {
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

    if let Some(include_descriptors) = params.include_descriptors {
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
