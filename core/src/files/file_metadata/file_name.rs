use super::page_type::PageType;
use anyhow::Result;

#[derive(Default, Clone)]
pub struct FileName(pub String);

impl TryFrom<String> for FileName {
  type Error = anyhow::Error;

  fn try_from(value: String) -> Result<Self> {
    match PageType::try_from(value.as_str()) {
      Ok(_) => Ok(Self(value)),
      Err(_) => Err(anyhow::Error::msg(format!("Invalid file name: {}", value))),
    }
  }
}

impl Into<String> for FileName {
  fn into(self) -> String {
    self.0
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
