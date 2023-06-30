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
use anyhow::Result;
use reqwest_middleware::ClientWithMiddleware;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_retry::{strategy::FibonacciBackoff, Retry};
use tracing::{info, instrument};

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
      self.process_queue_item(queue_item.clone()).await
    })
    .await?;
    self
      .crawler_interactor
      .increment_window_request_count()
      .await?;
    Ok(Some(result))
  }

  async fn wait(&self) {
    sleep(Duration::from_secs(self.settings.wait_time_seconds as u64)).await
  }

  pub async fn run(&self) -> Result<()> {
    loop {
      self.execute().await?;
      self.wait().await;
    }
  }
}
