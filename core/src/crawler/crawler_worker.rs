use super::{
  crawler_interactor::CrawlerInteractor, crawler_state_repository::CrawlerStatus,
  priority_queue::ItemKey,
};
use crate::{
  files::{
    file_interactor::FileInteractor,
    file_metadata::{file_metadata::FileMetadata, file_name::FileName},
  },
  settings::CrawlerSettings,
};
use anyhow::{anyhow, Result};
use reqwest_middleware::ClientWithMiddleware;
use std::sync::Arc;
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tokio_retry::{strategy::FibonacciBackoff, Retry};
use tracing::{instrument, warn};

#[derive(Error, Debug)]
#[error("Crawler worker error: {source}")]
pub struct CrawlerWorkerError {
  item_key: Option<ItemKey>,
  #[source]
  source: anyhow::Error,
}

pub struct CrawlerWorker {
  pub settings: CrawlerSettings,
  pub crawler_interactor: Arc<CrawlerInteractor>,
  pub file_interactor: Arc<FileInteractor>,
  pub client: ClientWithMiddleware,
}

impl CrawlerWorker {
  fn get_url(&self, file_name: &FileName) -> String {
    format!("https://rateyourmusic.com/{}", file_name.to_string())
  }

  #[instrument(skip(self))]
  async fn get_file_content(&self, file_name: &FileName) -> Result<String> {
    self
      .crawler_interactor
      .increment_window_request_count()
      .await?;

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

  #[instrument(skip(self))]
  async fn execute(&self) -> Result<Option<FileMetadata>, CrawlerWorkerError> {
    self
      .crawler_interactor
      .enforce_throttle()
      .await
      .map_err(|e| CrawlerWorkerError {
        item_key: None,
        source: e,
      })?;

    let status = self
      .crawler_interactor
      .get_status()
      .await
      .map_err(|e| CrawlerWorkerError {
        item_key: None,
        source: e,
      })?;
    if status == CrawlerStatus::Paused || status == CrawlerStatus::Throttled {
      return Ok(None);
    }

    let queue_item = match self.crawler_interactor.claim_item().await {
      Ok(Some(queue_item)) => queue_item,
      Ok(None) => return Ok(None),
      Err(e) => {
        return Err(CrawlerWorkerError {
          item_key: None,
          source: e,
        })
      }
    };

    let file_content = Retry::spawn(FibonacciBackoff::from_millis(500).take(5), || async {
      self.get_file_content(&queue_item.file_name).await
    })
    .await
    .map_err(|e| CrawlerWorkerError {
      item_key: Some(queue_item.item_key.clone()),
      source: anyhow!("Failed to get file content after 5 retries: {:?}", e),
    })?;

    let file_metadata = Retry::spawn(FibonacciBackoff::from_millis(500).take(5), || async {
      self
        .file_interactor
        .put_file(
          &queue_item.file_name,
          file_content.clone(),
          queue_item.correlation_id.clone(),
        )
        .await
    })
    .await
    .map_err(|e| CrawlerWorkerError {
      item_key: Some(queue_item.item_key.clone()),
      source: anyhow!("Failed to put file after 5 retries: {:?}", e),
    })?;

    self
      .crawler_interactor
      .delete_item(queue_item.item_key.clone())
      .await
      .map_err(|e| CrawlerWorkerError {
        item_key: Some(queue_item.item_key),
        source: e,
      })?;

    Ok(Some(file_metadata))
  }

  async fn wait(&self) {
    sleep(Duration::from_secs(self.settings.wait_time_seconds as u64)).await
  }

  pub async fn run(&self) -> Result<()> {
    loop {
      if let Err(e) = self.execute().await {
        warn!(
          e = &e.to_string().as_str(),
          "Failed to execute crawler worker"
        );
        if let Some(item_key) = e.item_key {
          self.crawler_interactor.handle_failure(item_key).await?;
        }
      }
      self.wait().await;
    }
  }
}
