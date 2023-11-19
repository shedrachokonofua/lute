use super::{
  crawler_interactor::CrawlerInteractor, crawler_state_repository::CrawlerStatus,
  priority_queue::QueueItem,
};
use crate::{
  files::{
    file_interactor::FileInteractor,
    file_metadata::{file_metadata::FileMetadata, file_name::FileName},
  },
  settings::CrawlerSettings,
};
use anyhow::{Error, Result};
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
    format!("http://www.rateyourmusic.com/{}", file_name.to_string())
  }

  #[instrument(skip(self))]
  async fn get_file_content(&self, file_name: &FileName) -> Result<String> {
    self
      .client
      .get(&self.get_url(file_name))
      .send()
      .await?
      .text()
      .await
      .map_err(|e| e.into())
  }

  #[instrument(skip(self))]
  async fn process_queue_item(&self, queue_item: QueueItem) -> Result<FileMetadata> {
    let metadata = self
      .file_interactor
      .put_file(
        &queue_item.file_name,
        self.get_file_content(&queue_item.file_name).await?,
        queue_item.correlation_id,
      )
      .await?;
    self
      .crawler_interactor
      .delete_item(queue_item.item_key)
      .await?;

    Ok(metadata)
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
    let result = Retry::spawn(FibonacciBackoff::from_millis(500).take(5), || async {
      info!(
        item = &queue_item.item_key.to_string(),
        "Processing queue item"
      );
      let file_metadata = self.process_queue_item(queue_item.clone()).await?;
      self
        .crawler_interactor
        .increment_window_request_count()
        .await?;
      Ok::<_, Error>(file_metadata)
    })
    .await
    .map_err(|e| {
      warn!(
        item = &queue_item.item_key.to_string(),
        e = &e.to_string().as_str(),
        "Failed to process queue item after 5 retries"
      );
      anyhow::anyhow!("Failed to process queue item after 5 retries: {:?}", e)
    })?;
    Ok(Some(result))
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
