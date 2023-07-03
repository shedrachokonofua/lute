use super::album_search_lookup_repository::{AlbumSearchLookup, AlbumSearchLookupRepository};
use crate::{
  crawler::crawler_interactor::CrawlerInteractor,
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, SubscriberContext},
  },
  settings::Settings,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

async fn handle_album_search_lookup_started(
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
    crawler_interactor.enqueue(QueuePushParameters {
      file_name: lookup,
      priority: (),
      deduplication_key: (),
      correlation_id: (),
      metadata: (),
    });
  }
  Ok(())
}

pub fn build_album_search_lookup_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Settings,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Vec<EventSubscriber> {
  vec![EventSubscriber {
    redis_connection_pool: Arc::clone(&redis_connection_pool),
    settings: settings.clone(),
    id: "handle_album_search_lookup_started".to_string(),
    concurrency: Some(250),
    stream: Stream::Lookup,
    handle: Arc::new(|context| {
      let crawler_interactor = Arc::clone(&crawler_interactor);
      Box::pin(async move { handle_album_search_lookup_started(context, crawler_interactor).await })
    }),
  }]
}
