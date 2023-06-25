use super::{
  crawler_interactor::CrawlerInteractor, crawler_worker::CrawlerWorker,
  priority_queue::PriorityQueue,
};
use crate::{files::file_interactor::FileInteractor, settings::Settings};
use anyhow::Result;
use r2d2::Pool;
use reqwest::Proxy;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::sync::Arc;
use tokio::task;

pub struct Crawler {
  settings: Settings,
  client: ClientWithMiddleware,
  pub crawler_interactor: Arc<CrawlerInteractor>,
  pub file_interactor: Arc<FileInteractor>,
}

impl Crawler {
  pub fn new(settings: Settings, redis_connection_pool: Arc<Pool<redis::Client>>) -> Result<Self> {
    let priority_queue = Arc::new(PriorityQueue::new(
      Arc::clone(&redis_connection_pool),
      settings.crawler.max_queue_size,
      settings.crawler.claim_ttl_seconds,
    ));
    let file_interactor =
      FileInteractor::new(settings.file.clone(), Arc::clone(&redis_connection_pool));
    let crawler_interactor = Arc::new(CrawlerInteractor::new(
      settings.crawler.clone(),
      file_interactor,
      Arc::clone(&redis_connection_pool),
      priority_queue.clone(),
    ));
    let file_interactor = Arc::new(FileInteractor::new(
      settings.file.clone(),
      Arc::clone(&redis_connection_pool),
    ));

    let proxy_settings = &settings.crawler.proxy;
    let base_client = reqwest::ClientBuilder::new()
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
      .map_err(|error| anyhow::Error::msg(error.to_string()))?;

    let client = ClientBuilder::new(base_client)
      .with(TracingMiddleware::default())
      .build();

    Ok(Self {
      client,
      settings,
      crawler_interactor,
      file_interactor,
    })
  }

  pub fn run(&self) -> Result<()> {
    for _ in 0..self.settings.crawler.pool_size {
      let crawler_worker = CrawlerWorker {
        settings: self.settings.crawler.clone(),
        crawler_interactor: self.crawler_interactor.clone(),
        file_interactor: self.file_interactor.clone(),
        client: self.client.clone(),
      };
      task::spawn(async move { crawler_worker.run().await });
    }
    Ok(())
  }
}
