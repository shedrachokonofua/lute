use super::{
  crawler_interactor::CrawlerInteractor, crawler_worker::CrawlerWorker,
  priority_queue::PriorityQueue,
};
use crate::{
  files::file_interactor::FileInteractor,
  settings::{Settings},
};
use anyhow::Result;
use r2d2::Pool;
use redis::Client;
use std::sync::Arc;
use tokio::task;

pub struct Crawler {
  settings: Settings,
  redis_connection_pool: Arc<Pool<Client>>,
  priority_queue: Arc<PriorityQueue>,
  pub crawler_interactor: Arc<CrawlerInteractor>,
  pub file_interactor: Arc<FileInteractor>,
}

impl Crawler {
  pub fn new(settings: Settings, redis_connection_pool: Arc<Pool<Client>>) -> Self {
    let priority_queue = Arc::new(PriorityQueue::new(
      redis_connection_pool.clone(),
      settings.crawler.max_queue_size,
      settings.crawler.claim_ttl_seconds,
    ));
    let crawler_interactor = Arc::new(CrawlerInteractor::new(
      settings.crawler.clone(),
      redis_connection_pool.clone(),
      priority_queue.clone(),
    ));
    let file_interactor = Arc::new(FileInteractor::new(
      settings.file.clone(),
      redis_connection_pool.clone(),
    ));

    Self {
      settings,
      redis_connection_pool,
      priority_queue,
      crawler_interactor,
      file_interactor,
    }
  }

  pub async fn run(&self) -> Result<()> {
    for _ in 1..self.settings.crawler.pool_size {
      let crawler_worker = CrawlerWorker::new(
        self.settings.crawler.clone(),
        self.crawler_interactor.clone(),
        self.file_interactor.clone(),
      );
      task::spawn(async move { crawler_worker.run().await });
    }
    Ok(())
  }
}
