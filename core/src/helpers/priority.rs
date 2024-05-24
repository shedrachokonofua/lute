use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Priority {
  Express = 0,
  High = 1,
  #[default]
  Standard = 2,
  Low = 3,
}

impl TryFrom<u32> for Priority {
  type Error = anyhow::Error;

  fn try_from(value: u32) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(Priority::Express),
      1 => Ok(Priority::High),
      2 => Ok(Priority::Standard),
      3 => Ok(Priority::Low),
      _ => bail!("Invalid priority value"),
    }
  }
}

impl TryFrom<f64> for Priority {
  type Error = anyhow::Error;

  fn try_from(value: f64) -> Result<Self, Self::Error> {
    Self::try_from(value as u32)
  }
}

impl ToString for Priority {
  fn to_string(&self) -> String {
    match self {
      Priority::Express => "express".to_string(),
      Priority::High => "high".to_string(),
      Priority::Standard => "standard".to_string(),
      Priority::Low => "low".to_string(),
    }
  }
}
