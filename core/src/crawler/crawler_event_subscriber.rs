use super::{crawler_interactor::CrawlerInteractor, priority_queue::QueuePushParameters};
use crate::{
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, SubscriberContext},
  },
  parser::parsed_file_data::ParsedFileData,
  settings::Settings,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

async fn crawl_chart_albums(
  context: SubscriberContext,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name: _,
    data: ParsedFileData::Chart(albums),
  } = context.payload.event
  {
    for album in albums {
      crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: album.file_name,
          priority: None,
          deduplication_key: None,
          correlation_id: None,
          metadata: None,
        })
        .await?;
    }
  }
  Ok(())
}

pub fn build_crawler_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Settings,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Vec<EventSubscriber> {
  vec![EventSubscriber {
    redis_connection_pool,
    settings,
    id: "crawl_chart_albums".to_string(),
    concurrency: Some(250),
    stream: Stream::Parser,
    handle: Arc::new(move |context| {
      let crawler_interactor = crawler_interactor.clone();
      Box::pin(async move { crawl_chart_albums(context, crawler_interactor).await })
    }),
  }]
}
