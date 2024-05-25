use anyhow::{bail, Error, Result};
use chrono::Duration;
use std::{str::FromStr, sync::Arc};

use crate::helpers::key_value_store::KeyValueStore;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CrawlerStatus {
  /**
   * Crawler is not processing the queue  
   */
  Paused,
  /**
   * Crawler is processing the queue, and is accepting new items
   */
  Running,
  /**
   * Crawler is not processing the queue, because the rate limit has been exceeded
   */
  Throttled,
}

impl ToString for CrawlerStatus {
  fn to_string(&self) -> String {
    match self {
      CrawlerStatus::Running => "running".to_string(),
      CrawlerStatus::Paused => "paused".to_string(),
      CrawlerStatus::Throttled => "throttled".to_string(),
    }
  }
}

impl FromStr for CrawlerStatus {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "running" => Ok(CrawlerStatus::Running),
      "paused" => Ok(CrawlerStatus::Paused),
      "throttled" => Ok(CrawlerStatus::Throttled),
      _ => bail!("Invalid status value"),
    }
  }
}

const THROTTLED_KEY: &str = "crawler:throttled";
const WINDOW_REQUEST_COUNT_KEY: &str = "crawler:window_request_count";

#[derive(Debug)]
pub struct CrawlerStateRepository {
  kv: Arc<KeyValueStore>,
}

impl CrawlerStateRepository {
  pub fn new(kv: Arc<KeyValueStore>) -> Self {
    Self { kv }
  }

  pub async fn is_throttled(&self) -> Result<bool> {
    let value = self.kv.get::<bool>(THROTTLED_KEY).await?.unwrap_or(false);
    Ok(value)
  }

  pub async fn set_throttled(&self, value: bool, duration: Option<Duration>) -> Result<()> {
    self
      .kv
      .set(
        THROTTLED_KEY,
        value,
        duration.map(|d| d.to_std()).transpose()?,
      )
      .await?;
    Ok(())
  }

  pub async fn get_window_request_count(&self) -> Result<u32> {
    let count = self
      .kv
      .get::<u32>(WINDOW_REQUEST_COUNT_KEY)
      .await?
      .unwrap_or(0);
    Ok(count)
  }

  pub async fn increment_window_request_count(&self) -> Result<()> {
    self.kv.increment(WINDOW_REQUEST_COUNT_KEY, 1).await?;
    Ok(())
  }

  pub async fn reset_window_request_count(&self) -> Result<()> {
    self.kv.delete(WINDOW_REQUEST_COUNT_KEY).await?;
    Ok(())
  }
}
