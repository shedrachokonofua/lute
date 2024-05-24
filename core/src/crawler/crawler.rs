use super::{
  crawler_state_repository::{CrawlerStateRepository, CrawlerStatus},
  priority_queue::{ClaimedQueueItem, ItemKey, QueueItem, QueuePushParameters},
};
use crate::{
  files::{file_interactor::FileInteractor, file_metadata::file_name::FileName},
  helpers::priority::Priority,
  scheduler::{
    job_name::JobName,
    scheduler::{JobParameters, JobParametersBuilder, Scheduler},
    scheduler_repository::Job,
  },
  settings::Settings,
};
use anyhow::Result;
use chrono::NaiveDateTime;
use reqwest::Proxy;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use rustis::{bb8::Pool, client::PooledClientManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawlJob {
  pub file_name: FileName,
  pub correlation_id: Option<String>,
  pub id: String,
  pub next_execution: NaiveDateTime,
  pub last_execution: Option<NaiveDateTime>,
  pub interval_seconds: Option<u32>,
  pub claimed_at: Option<NaiveDateTime>,
  pub priority: Priority,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawlJobPayload {
  file_name: FileName,
  correlation_id: Option<String>,
}

impl TryInto<CrawlJob> for Job {
  type Error = anyhow::Error;

  fn try_into(self) -> Result<CrawlJob> {
    let payload = self
      .payload
      .map(|p| serde_json::from_slice::<CrawlJobPayload>(&p))
      .transpose()?
      .ok_or_else(|| anyhow::Error::msg("Missing payload"))?;

    Ok(CrawlJob {
      file_name: payload.file_name,
      correlation_id: payload.correlation_id,
      id: self.id,
      next_execution: self.next_execution,
      last_execution: self.last_execution,
      interval_seconds: self.interval_seconds,
      claimed_at: self.claimed_at,
      priority: self.priority,
    })
  }
}

impl TryInto<ClaimedQueueItem> for Job {
  type Error = anyhow::Error;

  fn try_into(self) -> Result<ClaimedQueueItem> {
    let payload = self
      .payload
      .map(|p| serde_json::from_slice::<CrawlJobPayload>(&p))
      .transpose()?
      .ok_or_else(|| anyhow::Error::msg("Missing payload"))?;

    Ok(ClaimedQueueItem {
      item: QueueItem {
        item_key: ItemKey {
          enqueue_time: self.created_at,
          deduplication_key: self.id.clone(),
        },
        enqueue_time: self.created_at,
        deduplication_key: self.id,
        file_name: payload.file_name,
        priority: self.priority,
        correlation_id: payload.correlation_id,
        metadata: None,
      },
      claim_ttl_seconds: self
        .claimed_at
        .map(|d| d.signed_duration_since(self.created_at).num_seconds() as u32)
        .unwrap_or_default(),
    })
  }
}

impl TryInto<JobParameters> for QueuePushParameters {
  type Error = anyhow::Error;

  fn try_into(self) -> Result<JobParameters> {
    let payload = CrawlJobPayload {
      file_name: self.file_name.clone(),
      correlation_id: self.correlation_id,
    };

    Ok(
      JobParametersBuilder::default()
        .id(format!("crawl:{}", self.file_name.to_string()))
        .name(JobName::Crawl)
        .payload(serde_json::to_vec(&payload)?)
        .priority(self.priority.unwrap_or_default())
        .overwrite_existing(false)
        .build()?,
    )
  }
}

pub struct Crawler {
  settings: Arc<Settings>,
  client: ClientWithMiddleware,
  file_interactor: Arc<FileInteractor>,
  crawler_state_repository: CrawlerStateRepository,
  throttle_lock: Arc<Mutex<()>>,
  scheduler: Arc<Scheduler>,
}

pub struct CrawlerMonitor {
  pub status: CrawlerStatus,
  pub size: u32,
  pub claimed_item_count: u32,
  pub claimed_items: Vec<ClaimedQueueItem>,
  pub remaining_window_requests: u32,
  pub window_request_count: u32,
}

impl Crawler {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    file_interactor: Arc<FileInteractor>,
    scheduler: Arc<Scheduler>,
  ) -> Result<Self> {
    let mut base_client_builder = reqwest::ClientBuilder::new().danger_accept_invalid_certs(true);
    if let Some(proxy_settings) = &settings.crawler.proxy {
      base_client_builder = base_client_builder.proxy(
        Proxy::all(format!("{}:{}", proxy_settings.host, proxy_settings.port))?.basic_auth(
          proxy_settings.username.as_str(),
          proxy_settings.password.as_str(),
        ),
      );
    }
    let base_client = base_client_builder
      .build()
      .map_err(|error| anyhow::Error::msg(error.to_string()))?;

    let client = ClientBuilder::new(base_client)
      .with(TracingMiddleware::default())
      .build();

    Ok(Self {
      client,
      settings,
      file_interactor,
      crawler_state_repository: CrawlerStateRepository {
        redis_connection_pool,
      },
      throttle_lock: Arc::new(Mutex::new(())),
      scheduler,
    })
  }

  fn get_url(&self, file_name: &FileName) -> String {
    format!("https://rateyourmusic.com/{}", file_name.to_string())
  }

  #[instrument(skip(self))]
  pub async fn request(&self, file_name: &FileName) -> Result<String> {
    self.increment_window_request_count().await?;

    self
      .client
      .get(&self.get_url(file_name))
      .send()
      .await?
      .error_for_status()?
      .text()
      .await
      .map_err(|e| e.into())
  }

  pub async fn enqueue(&self, params: QueuePushParameters) -> Result<()> {
    self.scheduler.put(params.try_into()?).await?;
    Ok(())
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

  pub async fn set_status(&self, status: CrawlerStatus) -> Result<()> {
    self.crawler_state_repository.set_status(status).await
  }

  #[instrument(skip(self))]
  pub async fn get_status(&self) -> Result<CrawlerStatus> {
    self.crawler_state_repository.get_status().await
  }

  pub async fn empty_queue(&self) -> Result<()> {
    self.scheduler.delete_jobs_by_name(JobName::Crawl).await
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
    Ok(
      self
        .settings
        .crawler
        .rate_limit
        .max_requests
        .saturating_sub(self.get_window_request_count().await?),
    )
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
    let total = self.get_window_request_count().await?
      + (self
        .scheduler
        .count_claimed_jobs_by_name(JobName::Crawl)
        .await? as u32);
    Ok(total >= self.settings.crawler.rate_limit.max_requests)
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
    let size = self.scheduler.count_jobs_by_name(JobName::Crawl).await? as u32;
    let claimed_item_count = self
      .scheduler
      .count_claimed_jobs_by_name(JobName::Crawl)
      .await? as u32;
    let claimed_items = self
      .scheduler
      .find_claimed_jobs_by_name(JobName::Crawl)
      .await?
      .into_iter()
      .filter_map(|job| {
        job
          .try_into()
          .inspect_err(|e| tracing::error!("Error converting job to claimed item: {:?}", e))
          .ok()
      })
      .collect();
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
}
