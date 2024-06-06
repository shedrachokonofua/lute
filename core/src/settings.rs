use chrono::TimeDelta;
use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct RedisSettings {
  pub url: String,
  pub max_pool_size: u32,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct SqliteSettings {
  pub dir: String,
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
  pub proxy: Option<CrawlerProxySettings>,
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
  pub service_name: String,
  pub service_namespace: String,
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
pub struct OpenAISettings {
  pub api_key: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct VoyageAISettings {
  pub api_key: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct EmbeddingProviderSettings {
  pub openai: Option<OpenAISettings>,
  pub voyageai: Option<VoyageAISettings>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct ElasticSearchSettings {
  pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Settings {
  pub crawler: CrawlerSettings,
  pub file: FileSettings,
  pub port: u32,
  pub redis: RedisSettings,
  pub sqlite: SqliteSettings,
  pub spotify: SpotifySettings,
  pub tracing: TracingSettings,
  pub parser: ParserSettings,
  pub embedding_provider: EmbeddingProviderSettings,
  pub elasticsearch: ElasticSearchSettings,
}

impl Settings {
  pub fn new() -> Result<Self, config::ConfigError> {
    config::Config::builder()
      .add_source(config::Environment::default())
      .set_default("port", 80)?
      .set_default("file.ttl_days.artist", 7)?
      .set_default("file.ttl_days.album", 30)?
      .set_default("file.ttl_days.chart", 7)?
      .set_default("file.ttl_days.search", 7)?
      .set_default("file.content_store.key", None::<String>)?
      .set_default("file.content_store.secret", None::<String>)?
      .set_default("crawler.pool_size", 10)?
      .set_default(
        "crawler.claim_ttl_seconds",
        TimeDelta::try_minutes(2).unwrap().num_seconds(),
      )?
      .set_default("crawler.max_queue_size", 5000)?
      .set_default("crawler.wait_time_seconds", 5)?
      .set_default(
        "crawler.rate_limit.window_seconds",
        TimeDelta::try_days(1).unwrap().num_seconds(),
      )?
      .set_default("crawler.rate_limit.max_requests", 500)?
      .set_default("parser.concurrency", 20)?
      .set_default("parser.retry_concurrency", 20)?
      .set_default("tracing.service_name", "core")?
      .set_default("tracing.service_namespace", "lute")?
      .set_default("tracing.resource_labels", HashMap::<String, String>::new())?
      .set_default("sqlite.dir", env!("CARGO_MANIFEST_DIR"))?
      .build()?
      .try_deserialize()
  }
}
