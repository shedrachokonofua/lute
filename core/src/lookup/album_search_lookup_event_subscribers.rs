use super::album_search_lookup_repository::{
  AlbumSearchLookup, AlbumSearchLookupQuery, AlbumSearchLookupRepository,
};
use crate::{
  crawler::{
    crawler_interactor::CrawlerInteractor,
    priority_queue::{Priority, QueuePushParameters},
  },
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, SubscriberContext},
  },
  settings::Settings,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

async fn handle_event(
  context: SubscriberContext,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<()> {
  if let Event::LookupAlbumSearchStatusChanged {
    lookup: AlbumSearchLookup::Started { query },
  } = context.payload.event
  {
    let album_search_lookup_repository = AlbumSearchLookupRepository {
      redis_connection_pool: Arc::clone(&context.redis_connection_pool),
    };
    crawler_interactor
      .enqueue(QueuePushParameters {
        file_name: query.file_name(),
        priority: Some(Priority::Express),
        correlation_id: Some(format!("lookup:album_search:{}", query.to_encoded_string())),
        deduplication_key: None,
        metadata: None,
      })
      .await?;
    album_search_lookup_repository.put(&AlbumSearchLookup::SearchCrawling {
      album_search_file_name: query.file_name(),
      query: query,
      last_updated_at: chrono::Utc::now().naive_utc(),
    });
  }
  Ok(())
}

pub fn build_album_search_lookup_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Settings,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Vec<EventSubscriber> {
  vec![
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "lookup".to_string(),
      concurrency: Some(250),
      stream: Stream::Lookup,
      handle: Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&crawler_interactor);
        Box::pin(async move { handle_event(context, crawler_interactor).await })
      }),
    },
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "lookup".to_string(),
      concurrency: Some(1),
      stream: Stream::File,
      handle: Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&crawler_interactor);
        Box::pin(async move { handle_event(context, crawler_interactor).await })
      }),
    },
  ]
}
