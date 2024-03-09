use super::{crawler_interactor::CrawlerInteractor, crawler_state_repository::CrawlerStatus};
use crate::{
  files::{
    file_interactor::FileInteractor,
    file_metadata::{file_metadata::FileMetadata, file_name::FileName},
  },
  settings::CrawlerSettings,
};
use anyhow::Result;
use reqwest_middleware::ClientWithMiddleware;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_retry::{strategy::FibonacciBackoff, Retry};
use tracing::{info, instrument, warn};

#[derive(Debug)]
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
    let result = self
      .client
      .get(&self.get_url(file_name))
      .send()
      .await?
      .text()
      .await
      .map_err(|e| e.into());

    self
      .crawler_interactor
      .increment_window_request_count()
      .await?;

    result
  }

  #[instrument(skip(self))]
  async fn execute(&self) -> Result<Option<FileMetadata>> {
    self.crawler_interactor.enforce_throttle().await?;
    let status = self.crawler_interactor.get_status().await?;
    if status == CrawlerStatus::Paused || status == CrawlerStatus::Throttled {
      return Ok(None);
    }
    let queue_item = self.crawler_interactor.claim_item().await?;
    if queue_item.is_none() {
      return Ok(None);
    }
    let queue_item = queue_item.unwrap();

    let file_content = Retry::spawn(FibonacciBackoff::from_millis(500).take(5), || async {
      info!(
        item = &queue_item.item_key.to_string(),
        "Getting file content for queue item"
      );
      self.get_file_content(&queue_item.file_name).await
    })
    .await
    .map_err(|e| {
      warn!(
        item = &queue_item.item_key.to_string(),
        e = &e.to_string().as_str(),
        "Failed to get file content after 5 retries"
      );
      anyhow::anyhow!("Failed to get file content after 5 retries: {:?}", e)
    })?;

    let file_metadata = Retry::spawn(FibonacciBackoff::from_millis(500).take(5), || async {
      info!(
        item = &queue_item.item_key.to_string(),
        "Putting file content for queue item"
      );
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
    .map_err(|e| {
      warn!(
        item = &queue_item.item_key.to_string(),
        e = &e.to_string().as_str(),
        "Failed to put file content after 5 retries"
      );
      anyhow::anyhow!("Failed to put file content after 5 retries: {:?}", e)
    })?;

    self
      .crawler_interactor
      .delete_item(queue_item.item_key)
      .await?;

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
      }
      self.wait().await;
    }
  }
}
