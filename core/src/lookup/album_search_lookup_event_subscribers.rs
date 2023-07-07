use super::{
  album_search_lookup::{
    get_query_from_album_search_correlation_id, is_album_search_correlation_id, AlbumSearchLookup,
    AlbumSearchLookupStep,
  },
  album_search_lookup_repository::AlbumSearchLookupRepository,
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
  files::file_metadata::page_type::PageType,
  parser::parsed_file_data::ParsedFileData,
  settings::Settings,
};
use anyhow::Result;
use futures::future::BoxFuture;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

async fn handle_event(
  context: SubscriberContext,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<()> {
  if let Some(correlation_id) = &context.payload.correlation_id {
    if !is_album_search_correlation_id(correlation_id) {
      return Ok(());
    }

    let album_search_lookup_repository = AlbumSearchLookupRepository {
      redis_connection_pool: Arc::clone(&context.redis_connection_pool),
    };

    if let Event::LookupAlbumSearchStatusChanged {
      lookup: AlbumSearchLookup::Started { query },
    } = context.payload.event
    {
      let correlation_id = context.payload.correlation_id.unwrap();
      crawler_interactor
        .enqueue(QueuePushParameters {
          file_name: query.file_name(),
          priority: Some(Priority::Express),
          correlation_id: Some(correlation_id.clone()),
          deduplication_key: None,
          metadata: None,
        })
        .await?;
      album_search_lookup_repository
        .put(&AlbumSearchLookup::SearchCrawling {
          album_search_file_name: query.file_name(),
          query: query,
          last_updated_at: chrono::Utc::now().naive_utc(),
          file_processing_correlation_id: correlation_id.clone(),
        })
        .await?;
    } else if let Event::FileSaved {
      file_id: _,
      file_name,
    } = context.payload.event
    {
      let correlation_id = context.payload.correlation_id.unwrap();
      let query = get_query_from_album_search_correlation_id(&correlation_id)?;
      let lookup = album_search_lookup_repository.get(&query).await?;

      match file_name.page_type() {
        PageType::AlbumSearchResult => {
          if lookup.can_transition(AlbumSearchLookupStep::SearchParsing, &correlation_id) {
            album_search_lookup_repository
              .put(&AlbumSearchLookup::SearchParsing {
                album_search_file_name: file_name,
                query: lookup.query().clone(),
                last_updated_at: chrono::Utc::now().naive_utc(),
                file_processing_correlation_id: correlation_id.clone(),
              })
              .await?;
          }
        }
        PageType::Album => {
          if lookup.can_transition(AlbumSearchLookupStep::AlbumParsing, &correlation_id) {
            album_search_lookup_repository
              .put(&AlbumSearchLookup::AlbumParsing {
                album_search_file_name: file_name,
                query: lookup.query().clone(),
                last_updated_at: chrono::Utc::now().naive_utc(),
                file_processing_correlation_id: correlation_id.clone(),
                parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
              })
              .await?;
          }
        }
        _ => {}
      };
    } else if let Event::FileParsed {
      file_name, data, ..
    } = context.payload.event
    {
      let correlation_id = context.payload.correlation_id.unwrap();
      let query = get_query_from_album_search_correlation_id(&correlation_id)?;
      let lookup = album_search_lookup_repository.get(&query).await?;

      match data {
        ParsedFileData::AlbumSearchResult(album_search_result) => {
          if lookup.can_transition(AlbumSearchLookupStep::AlbumCrawling, &correlation_id) {
            crawler_interactor
              .enqueue(QueuePushParameters {
                file_name: album_search_result.file_name.clone(),
                priority: Some(Priority::Express),
                correlation_id: Some(correlation_id.clone()),
                deduplication_key: None,
                metadata: None,
              })
              .await?;
            album_search_lookup_repository
              .put(&AlbumSearchLookup::AlbumCrawling {
                album_search_file_name: file_name,
                query: lookup.query().clone(),
                last_updated_at: chrono::Utc::now().naive_utc(),
                file_processing_correlation_id: correlation_id.clone(),
                parsed_album_search_result: album_search_result,
              })
              .await?;
          }
        }
        ParsedFileData::Album(album) => {
          if lookup.can_transition(AlbumSearchLookupStep::AlbumParsed, &correlation_id) {
            album_search_lookup_repository
              .put(&AlbumSearchLookup::AlbumParsed {
                album_search_file_name: file_name,
                query: lookup.query().clone(),
                last_updated_at: chrono::Utc::now().naive_utc(),
                file_processing_correlation_id: correlation_id.clone(),
                parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
                parsed_album: album,
              })
              .await?;
          }
        }
        _ => {}
      };
    } else if let Event::FileParseFailed {
      file_name,
      error,
      file_id: _,
    } = context.payload.event
    {
      let correlation_id = context.payload.correlation_id.unwrap();
      let query = get_query_from_album_search_correlation_id(&correlation_id)?;
      let lookup = album_search_lookup_repository.get(&query).await?;

      match file_name.page_type() {
        PageType::AlbumSearchResult => {
          if lookup.can_transition(AlbumSearchLookupStep::SearchParseFailed, &correlation_id) {
            album_search_lookup_repository
              .put(&AlbumSearchLookup::SearchParseFailed {
                album_search_file_name: file_name,
                query: lookup.query().clone(),
                last_updated_at: chrono::Utc::now().naive_utc(),
                file_processing_correlation_id: correlation_id.clone(),
                album_search_file_parse_error: error,
              })
              .await?;
          }
        }
        PageType::Album => {
          if lookup.can_transition(AlbumSearchLookupStep::AlbumParseFailed, &correlation_id) {
            album_search_lookup_repository
              .put(&AlbumSearchLookup::AlbumParseFailed {
                album_search_file_name: file_name,
                query: lookup.query().clone(),
                last_updated_at: chrono::Utc::now().naive_utc(),
                file_processing_correlation_id: correlation_id.clone(),
                parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
                album_file_parse_error: error,
              })
              .await?;
          }
        }
        _ => {}
      }
    }
  }
  Ok(())
}

pub fn build_album_search_lookup_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Settings,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Vec<EventSubscriber> {
  let lookup_event_handler: Arc<
    dyn Fn(SubscriberContext) -> BoxFuture<'static, Result<()>> + Send + Sync,
  > = Arc::new(move |context| {
    let crawler_interactor = Arc::clone(&crawler_interactor);
    Box::pin(async move { handle_event(context, crawler_interactor).await })
  });
  vec![
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "lookup".to_string(),
      concurrency: Some(250),
      stream: Stream::Lookup,
      handle: Arc::clone(&lookup_event_handler),
    },
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "lookup".to_string(),
      concurrency: Some(250),
      stream: Stream::File,
      handle: Arc::clone(&lookup_event_handler),
    },
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "lookup".to_string(),
      concurrency: Some(250),
      stream: Stream::Parser,
      handle: Arc::clone(&lookup_event_handler),
    },
  ]
}
