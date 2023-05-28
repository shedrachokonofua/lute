use chrono::{DateTime, Utc};
use std::{str::FromStr, time::SystemTime};

#[derive(Debug, Clone)]
pub struct FileTimestamp(pub DateTime<Utc>);

impl FileTimestamp {
  pub fn now() -> Self {
    Self::default()
  }
}

impl Default for FileTimestamp {
  fn default() -> Self {
    FileTimestamp(Utc::now())
  }
}

impl FromStr for FileTimestamp {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(FileTimestamp(
      DateTime::parse_from_rfc2822(s)?.with_timezone(&Utc),
    ))
  }
}

impl ToString for FileTimestamp {
  fn to_string(&self) -> String {
    self.0.to_rfc2822()
  }
}

impl From<SystemTime> for FileTimestamp {
  fn from(system_time: SystemTime) -> Self {
    FileTimestamp(system_time.into())
  }
}

impl From<DateTime<Utc>> for FileTimestamp {
  fn from(datetime: DateTime<Utc>) -> Self {
    FileTimestamp(datetime)
  }
}

impl Into<DateTime<Utc>> for FileTimestamp {
  fn into(self) -> DateTime<Utc> {
    self.0
  }
}
