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

impl FileName {
  pub fn page_type(&self) -> PageType {
    PageType::try_from(self.0.as_str()).unwrap()
  }
}
