use super::{
  crawler_interactor::CrawlerInteractor, crawler_worker::CrawlerWorker,
  redis_priority_queue::RedisPriorityQueue,
};
use crate::{files::file_interactor::FileInteractor, settings::Settings, sqlite::SqliteConnection};
use anyhow::Result;
use reqwest::Proxy;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tokio::task;

pub struct Crawler {
  settings: Arc<Settings>,
  client: ClientWithMiddleware,
  pub crawler_interactor: Arc<CrawlerInteractor>,
  pub file_interactor: Arc<FileInteractor>,
}

impl Crawler {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    sqlite_connection: Arc<SqliteConnection>,
  ) -> Result<Self> {
    let priority_queue = Arc::new(RedisPriorityQueue::new(
      Arc::clone(&redis_connection_pool),
      settings.crawler.max_queue_size,
      settings.crawler.claim_ttl_seconds,
    ));
    let file_interactor = FileInteractor::new(
      Arc::clone(&settings),
      Arc::clone(&redis_connection_pool),
      Arc::clone(&sqlite_connection),
    );
    let crawler_interactor = Arc::new(CrawlerInteractor::new(
      Arc::clone(&settings),
      file_interactor,
      Arc::clone(&redis_connection_pool),
      priority_queue,
    ));
    let file_interactor = Arc::new(FileInteractor::new(
      Arc::clone(&settings),
      Arc::clone(&redis_connection_pool),
      Arc::clone(&sqlite_connection),
    ));

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
