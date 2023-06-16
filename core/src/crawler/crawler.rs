use super::{
  crawler_interactor::CrawlerInteractor, crawler_worker::CrawlerWorker,
  priority_queue::PriorityQueue,
};
use crate::{files::file_interactor::FileInteractor, settings::Settings};
use anyhow::Result;
use r2d2::Pool;
use reqwest::{ClientBuilder, Proxy};
use std::sync::Arc;
use tokio::task;

pub struct Crawler {
  settings: Settings,
  redis_connection_pool: Arc<Pool<redis::Client>>,
  priority_queue: Arc<PriorityQueue>,
  pub crawler_interactor: Arc<CrawlerInteractor>,
  pub file_interactor: Arc<FileInteractor>,
}

impl Crawler {
  pub fn new(settings: Settings, redis_connection_pool: Arc<Pool<redis::Client>>) -> Self {
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

  pub fn client(&self) -> Result<reqwest::Client> {
    let proxy_settings = &self.settings.crawler.proxy;
    ClientBuilder::new()
      .proxy(
        Proxy::all(format!(
          "https://{}:{}",
          proxy_settings.host, proxy_settings.port
        ))?
        .basic_auth(
          proxy_settings.username.as_str(),
          proxy_settings.password.as_str(),
        ),
      )
      .danger_accept_invalid_certs(true)
      .build()
      .map_err(|error| anyhow::Error::msg(error.to_string()))
  }

  pub fn run(&self) -> Result<()> {
    let client = self.client()?;
    for _ in 1..self.settings.crawler.pool_size {
      let crawler_worker = CrawlerWorker {
        settings: self.settings.crawler.clone(),
        crawler_interactor: self.crawler_interactor.clone(),
        file_interactor: self.file_interactor.clone(),
        client: client.clone(),
      };
      task::spawn(async move { crawler_worker.run().await });
    }
    Ok(())
  }
}
