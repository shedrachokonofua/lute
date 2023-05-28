#[derive(Debug, Clone, Default, serde_derive::Deserialize, PartialEq, Eq)]
pub struct RedisSettings {
  pub url: String,
  pub max_pool_size: u32,
}

#[derive(Debug, Clone, Default, serde_derive::Deserialize, PartialEq, Eq)]

pub struct FileTtlDaysSettings {
  pub artist: u32,
  pub album: u32,
  pub search: u32,
  pub chart: u32,
}

#[derive(Debug, Clone, Default, serde_derive::Deserialize, PartialEq, Eq)]
pub struct FileSettings {
  pub ttl_days: FileTtlDaysSettings,
}

#[derive(Debug, Clone, Default, serde_derive::Deserialize, PartialEq, Eq)]
pub struct Settings {
  pub redis: RedisSettings,
  pub file: FileSettings,
}

impl Settings {
  pub fn new() -> Result<Self, config::ConfigError> {
    let s = config::Config::builder()
      .add_source(config::Environment::default())
      .set_default("file.ttl_days.artist", 14)?
      .set_default("file.ttl_days.album", 7)?
      .set_default("file.ttl_days.chart", 7)?
      .set_default("file.ttl_days.search", 1)?
      .build()?;

    Ok(s.try_deserialize()?)
  }
}
