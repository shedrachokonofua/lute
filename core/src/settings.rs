use std::collections::HashMap;

use chrono::Duration;
use serde_derive::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct RedisSettings {
  pub url: String,
  pub max_pool_size: u32,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct FileTtlDaysSettings {
  pub artist: u32,
  pub album: u32,
  pub search: u32,
  pub chart: u32,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct ContentStoreSettings {
  pub region: String,
  pub endpoint: String,
  pub key: Option<String>,
  pub secret: Option<String>,
  pub bucket: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct FileSettings {
  pub ttl_days: FileTtlDaysSettings,
  pub content_store: ContentStoreSettings,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct CrawlerRateLimitSettings {
  pub window_seconds: u32,
  pub max_requests: u32,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct CrawlerProxySettings {
  pub host: String,
  pub port: u32,
  pub username: String,
  pub password: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct CrawlerSettings {
  pub proxy: CrawlerProxySettings,
  pub pool_size: u32,
  pub claim_ttl_seconds: u32,
  pub max_queue_size: u32,
  pub wait_time_seconds: u32,
  pub rate_limit: CrawlerRateLimitSettings,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct TracingSettings {
  pub otel_collector_endpoint: String,
  pub host_name: String,
  pub name: String,
  pub namespace: String,
  pub resource_labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct SpotifySettings {
  pub client_id: String,
  pub client_secret: String,
  pub redirect_uri: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct ParserSettings {
  pub concurrency: u16,
  pub retry_concurrency: u16,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Settings {
  pub crawler: CrawlerSettings,
  pub file: FileSettings,
  pub port: u32,
  pub redis: RedisSettings,
  pub spotify: SpotifySettings,
  pub tracing: TracingSettings,
  pub parser: ParserSettings,
}

impl Settings {
  pub fn new() -> Result<Self, config::ConfigError> {
    config::Config::builder()
      .add_source(config::Environment::default())
      .set_default("port", 80)?
      .set_default("file.ttl_days.artist", 14)?
      .set_default("file.ttl_days.album", 7)?
      .set_default("file.ttl_days.chart", 7)?
      .set_default("file.ttl_days.search", 1)?
      .set_default("file.content_store.key", None::<String>)?
      .set_default("file.content_store.secret", None::<String>)?
      .set_default("crawler.pool_size", 10)?
      .set_default(
        "crawler.claim_ttl_seconds",
        Duration::minutes(5).num_seconds(),
      )?
      .set_default("crawler.max_queue_size", 5000)?
      .set_default("crawler.wait_time_seconds", 5)?
      .set_default(
        "crawler.rate_limit.window_seconds",
        Duration::days(1).num_seconds(),
      )?
      .set_default("crawler.rate_limit.max_requests", 2000)?
      .set_default("parser.concurrency", 20)?
      .set_default("parser.retry_concurrency", 20)?
      .set_default("tracing.name", "core")?
      .set_default("tracing.namespace", "lute")?
      .set_default("tracing.labels", HashMap::<String, String>::new())?
      .build()?
      .try_deserialize()
  }
}
