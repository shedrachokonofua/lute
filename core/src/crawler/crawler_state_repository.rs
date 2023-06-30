use anyhow::{bail, Error, Result};
use rustis::{bb8::Pool, client::PooledClientManager, commands::StringCommands};
use std::{str::FromStr, sync::Arc};

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
   * Crawler is processing the queue, but is not accepting new items
   */
  Draining,
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
      CrawlerStatus::Draining => "draining".to_string(),
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
      "draining" => Ok(CrawlerStatus::Draining),
      "throttled" => Ok(CrawlerStatus::Throttled),
      _ => bail!("Invalid status value"),
    }
  }
}

#[derive(Debug)]
pub struct CrawlerStateRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl CrawlerStateRepository {
  pub fn status_key(&self) -> String {
    "crawler:status".to_string()
  }

  pub fn window_request_count_key(&self) -> String {
    "crawler:window_request_count".to_string()
  }

  pub async fn get_status(&self) -> Result<CrawlerStatus> {
    let connection = self.redis_connection_pool.get().await?;
    let status: Option<String> = connection.get(self.status_key()).await?;
    Ok(
      status
        .map(|status| CrawlerStatus::from_str(&status).unwrap())
        .unwrap_or(CrawlerStatus::Running),
    )
  }

  pub async fn set_status(&self, status: CrawlerStatus) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection
      .set(self.status_key(), status.to_string())
      .await?;
    Ok(())
  }

  pub async fn get_window_request_count(&self) -> Result<u32> {
    let connection = self.redis_connection_pool.get().await?;
    let count: Option<u32> = connection.get(self.window_request_count_key()).await?;
    Ok(count.unwrap_or(0))
  }

  pub async fn increment_window_request_count(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection.incr(self.window_request_count_key()).await?;
    Ok(())
  }

  pub async fn reset_window_request_count(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection.set(self.window_request_count_key(), 0).await?;
    Ok(())
  }
}
