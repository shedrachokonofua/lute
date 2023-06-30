use super::{
  crawler_state_repository::{CrawlerStateRepository, CrawlerStatus},
  priority_queue::{ClaimedQueueItem, ItemKey, PriorityQueue, QueueItem, QueuePushParameters},
};
use crate::{files::file_interactor::FileInteractor, settings::CrawlerSettings};
use anyhow::{bail, Result};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

pub struct CrawlerMonitor {
  pub status: CrawlerStatus,
  pub size: u32,
  pub claimed_item_count: u32,
  pub claimed_items: Vec<ClaimedQueueItem>,
  pub remaining_window_requests: u32,
  pub window_request_count: u32,
}

#[derive(Debug)]
pub struct CrawlerInteractor {
  settings: CrawlerSettings,
  file_interactor: FileInteractor,
  crawler_state_repository: CrawlerStateRepository,
  priority_queue: Arc<PriorityQueue>,
  throttle_lock: Mutex<()>,
}

impl CrawlerInteractor {
  pub fn new(
    settings: CrawlerSettings,
    file_interactor: FileInteractor,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    priority_queue: Arc<PriorityQueue>,
  ) -> Self {
    Self {
      settings,
      file_interactor,
      priority_queue,
      crawler_state_repository: CrawlerStateRepository {
        redis_connection_pool,
      },
      throttle_lock: Mutex::new(()),
    }
  }

  pub async fn set_status(&self, status: CrawlerStatus) -> Result<()> {
    self.crawler_state_repository.set_status(status).await
  }

  #[instrument(skip(self))]
  pub async fn get_status(&self) -> Result<CrawlerStatus> {
    self.crawler_state_repository.get_status().await
  }

  pub async fn enqueue(&self, params: QueuePushParameters) -> Result<()> {
    if self.get_status().await? == CrawlerStatus::Draining {
      bail!("Crawler is draining")
    }

    self.priority_queue.push(params).await
  }

  pub async fn enqueue_if_stale(&self, params: QueuePushParameters) -> Result<()> {
    if self
      .file_interactor
      .is_file_stale(&params.file_name)
      .await?
    {
      self.enqueue(params).await?
    }
    Ok(())
  }

  pub async fn empty_queue(&self) -> Result<()> {
    self.priority_queue.empty().await
  }

  pub async fn get_window_request_count(&self) -> Result<u32> {
    self
      .crawler_state_repository
      .get_window_request_count()
      .await
  }

  pub async fn increment_window_request_count(&self) -> Result<()> {
    self
      .crawler_state_repository
      .increment_window_request_count()
      .await
  }

  pub async fn remaining_window_requests(&self) -> Result<u32> {
    Ok(self.settings.rate_limit.max_requests - self.get_window_request_count().await?)
  }

  pub async fn reset_window_request_count(&self) -> Result<()> {
    self
      .crawler_state_repository
      .reset_window_request_count()
      .await
  }

  pub async fn remove_throttle(&self) -> Result<()> {
    self
      .crawler_state_repository
      .reset_window_request_count()
      .await?;
    self.set_status(CrawlerStatus::Running).await
  }

  pub async fn should_throttle(&self) -> Result<bool> {
    if self.get_status().await? == CrawlerStatus::Throttled {
      return Ok(false);
    }
    let total =
      self.get_window_request_count().await? + self.priority_queue.get_claimed_item_count().await?;
    Ok(total >= self.settings.rate_limit.max_requests)
  }

  #[instrument(skip(self))]
  pub async fn enforce_throttle(&self) -> Result<()> {
    let _ = self.throttle_lock.lock().await;
    if self.should_throttle().await? {
      self.set_status(CrawlerStatus::Throttled).await?;
    }
    Ok(())
  }

  pub async fn get_monitor(&self) -> Result<CrawlerMonitor> {
    let status = self.get_status().await?;
    let size = self.priority_queue.get_size().await?;
    let claimed_item_count = self.priority_queue.get_claimed_item_count().await?;
    let claimed_items = self.priority_queue.get_claimed_items().await?;
    let remaining_window_requests = self.remaining_window_requests().await?;
    let window_request_count = self.get_window_request_count().await?;

    Ok(CrawlerMonitor {
      status,
      size,
      claimed_item_count,
      claimed_items,
      remaining_window_requests,
      window_request_count,
    })
  }

  pub async fn claim_item(&self) -> Result<Option<QueueItem>> {
    self.priority_queue.claim_item().await
  }

  pub async fn delete_item(&self, item_key: ItemKey) -> Result<()> {
    self.priority_queue.delete_item(item_key).await
  }
}
