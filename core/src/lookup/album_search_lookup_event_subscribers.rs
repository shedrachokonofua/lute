use super::{
  album_search_lookup::{
    get_query_from_album_search_correlation_id, is_album_search_correlation_id, AlbumSearchLookup,
    AlbumSearchLookupStep,
  },
  lookup_interactor::LookupInteractor,
};
use crate::{
  albums::album_read_model_repository::{AlbumReadModel, AlbumReadModelRepository},
  crawler::{
    crawler_interactor::CrawlerInteractor,
    priority_queue::{Priority, QueuePushParameters},
  },
  events::{
    event::{Event, EventPayload, Stream},
    event_publisher::EventPublisher,
    event_subscriber::{EventSubscriber, SubscriberContext},
  },
  files::file_metadata::page_type::PageType,
  parser::parsed_file_data::{
    ParsedAlbum, ParsedArtistReference, ParsedCredit, ParsedFileData, ParsedTrack,
  },
  settings::Settings,
};
use anyhow::Result;
use chrono::Utc;
use futures::future::BoxFuture;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tracing::info;

impl From<AlbumReadModel> for ParsedAlbum {
  fn from(album: AlbumReadModel) -> Self {
    Self {
      name: album.name,
      rating: album.rating,
      rating_count: album.rating_count,
      artists: album
        .artists
        .iter()
        .map(|artist| ParsedArtistReference {
          name: artist.name.clone(),
          file_name: artist.file_name.clone(),
        })
        .collect::<Vec<ParsedArtistReference>>(),
      primary_genres: album.primary_genres,
      secondary_genres: album.secondary_genres,
      descriptors: album.descriptors,
      tracks: album
        .tracks
        .iter()
        .map(|track| ParsedTrack {
          name: track.name.clone(),
          duration_seconds: track.duration_seconds,
          rating: track.rating,
          position: track.position.clone(),
        })
        .collect::<Vec<ParsedTrack>>(),
      release_date: album.release_date,
      languages: album.languages,
      credits: album
        .credits
        .iter()
        .map(|credit| ParsedCredit {
          artist: ParsedArtistReference {
            name: credit.artist.name.clone(),
            file_name: credit.artist.file_name.clone(),
          },
          roles: credit.roles.clone(),
        })
        .collect::<Vec<ParsedCredit>>(),
    }
  }
}

async fn handle_file_processing_event(
  context: SubscriberContext,
  lookup_interactor: LookupInteractor,
  event_publisher: EventPublisher,
) -> Result<()> {
  if context.payload.correlation_id.is_none() {
    return Ok(());
  }
  let correlation_id = context.payload.correlation_id.unwrap();
  if is_album_search_correlation_id(&correlation_id) {
    let query = get_query_from_album_search_correlation_id(&correlation_id)?;
    let lookup = lookup_interactor.get_album_search_lookup(&query).await?;

    let next_lookup = match context.payload.event {
      Event::FileSaved { file_name, .. } => match file_name.page_type() {
        PageType::AlbumSearchResult => {
          if lookup.can_transition(AlbumSearchLookupStep::SearchParsing, &correlation_id) {
            Some(AlbumSearchLookup::SearchParsing {
              album_search_file_name: file_name,
              query: lookup.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
            })
          } else {
            None
          }
        }
        PageType::Album => {
          if lookup.can_transition(AlbumSearchLookupStep::AlbumParsing, &correlation_id) {
            Some(AlbumSearchLookup::AlbumParsing {
              album_search_file_name: file_name,
              query: lookup.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
            })
          } else {
            None
          }
        }
        _ => None,
      },
      Event::FileParsed {
        file_name, data, ..
      } => match data {
        ParsedFileData::AlbumSearchResult(album_search_result) => {
          if lookup.can_transition(AlbumSearchLookupStep::SearchParsed, &correlation_id) {
            Some(AlbumSearchLookup::SearchParsed {
              album_search_file_name: file_name,
              query: lookup.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: album_search_result,
            })
          } else {
            None
          }
        }
        ParsedFileData::Album(album) => {
          if lookup.can_transition(AlbumSearchLookupStep::AlbumParsed, &correlation_id) {
            Some(AlbumSearchLookup::AlbumParsed {
              album_search_file_name: file_name,
              query: lookup.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
              parsed_album: album,
            })
          } else {
            None
          }
        }
        _ => None,
      },
      Event::FileParseFailed {
        file_name, error, ..
      } => match file_name.page_type() {
        PageType::AlbumSearchResult => {
          if lookup.can_transition(AlbumSearchLookupStep::SearchParseFailed, &correlation_id) {
            Some(AlbumSearchLookup::SearchParseFailed {
              album_search_file_name: file_name,
              query: lookup.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              album_search_file_parse_error: error,
            })
          } else {
            None
          }
        }
        PageType::Album => {
          if lookup.can_transition(AlbumSearchLookupStep::AlbumParseFailed, &correlation_id) {
            Some(AlbumSearchLookup::AlbumParseFailed {
              album_search_file_name: file_name,
              query: lookup.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
              album_file_parse_error: error,
            })
          } else {
            None
          }
        }
        _ => None,
      },
      _ => None,
    };

    if let Some(next_lookup) = next_lookup {
      event_publisher
        .publish(
          Stream::Lookup,
          EventPayload {
            event: Event::LookupAlbumSearchUpdated {
              lookup: next_lookup.clone(),
            },
            correlation_id: Some(correlation_id),
            metadata: None,
          },
        )
        .await?;
    }
  } else if let Event::FileParsed {
    file_name,
    data: ParsedFileData::Album(album),
    ..
  } = context.payload.event
  {
    let lookups = lookup_interactor
      .find_many_album_search_lookups_by_album_file_name(&file_name)
      .await?;
    for lookup in lookups {
      info!(
        file_name = file_name.to_string(),
        "Found album search lookup for album file name"
      );
      event_publisher
        .publish(
          Stream::Lookup,
          EventPayload {
            event: Event::LookupAlbumSearchUpdated {
              lookup: AlbumSearchLookup::AlbumParsed {
                album_search_file_name: lookup.album_search_file_name().unwrap(),
                query: lookup.query().clone(),
                last_updated_at: chrono::Utc::now().naive_utc(),
                file_processing_correlation_id: lookup.file_processing_correlation_id().clone(),
                parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
                parsed_album: album.clone(),
              },
            },
            correlation_id: Some(lookup.file_processing_correlation_id().clone()),
            metadata: None,
          },
        )
        .await?;
    }
  }
  Ok(())
}

async fn handle_lookup_event(
  context: SubscriberContext,
  crawler_interactor: Arc<CrawlerInteractor>,
  lookup_interactor: LookupInteractor,
  event_publisher: EventPublisher,
  album_read_model_repository: AlbumReadModelRepository,
) -> Result<()> {
  if context.payload.correlation_id.is_none() {
    return Ok(());
  }
  let correlation_id = context.payload.correlation_id.unwrap();

  if !is_album_search_correlation_id(&correlation_id) {
    return Ok(());
  }

  if let Event::LookupAlbumSearchUpdated { lookup } = context.payload.event {
    lookup_interactor.put_album_search_lookup(&lookup).await?;
    if let AlbumSearchLookup::Started { query, .. } = lookup {
      crawler_interactor
        .enqueue(QueuePushParameters {
          file_name: query.file_name(),
          priority: Some(Priority::High),
          deduplication_key: None,
          correlation_id: Some(correlation_id.clone()),
          metadata: None,
        })
        .await?;
      lookup_interactor
        .put_album_search_lookup(&AlbumSearchLookup::SearchCrawling {
          query: query.clone(),
          last_updated_at: Utc::now().naive_utc(),
          album_search_file_name: query.file_name(),
          file_processing_correlation_id: correlation_id.clone(),
        })
        .await?;
    } else if let AlbumSearchLookup::SearchParsed {
      parsed_album_search_result,
      query,
      album_search_file_name,
      file_processing_correlation_id,
      ..
    } = lookup
    {
      match album_read_model_repository
        .find(&parsed_album_search_result.file_name)
        .await?
      {
        Some(album) => {
          event_publisher
            .publish(
              Stream::Lookup,
              EventPayload {
                event: Event::LookupAlbumSearchUpdated {
                  lookup: AlbumSearchLookup::AlbumParsed {
                    query,
                    last_updated_at: Utc::now().naive_utc(),
                    album_search_file_name,
                    file_processing_correlation_id,
                    parsed_album_search_result,
                    parsed_album: album.into(),
                  },
                },
                correlation_id: Some(correlation_id.clone()),
                metadata: None,
              },
            )
            .await?;
        }
        None => {
          crawler_interactor
            .enqueue(QueuePushParameters {
              file_name: parsed_album_search_result.file_name.clone(),
              priority: Some(Priority::High),
              deduplication_key: None,
              correlation_id: Some(correlation_id.clone()),
              metadata: None,
            })
            .await?;
          lookup_interactor
            .put_album_search_lookup(&AlbumSearchLookup::AlbumCrawling {
              query,
              last_updated_at: Utc::now().naive_utc(),
              album_search_file_name,
              file_processing_correlation_id,
              parsed_album_search_result,
            })
            .await?;
        }
      }
    }
  }
  Ok(())
}

pub fn build_album_search_lookup_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Arc<Settings>,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Vec<EventSubscriber> {
  let file_processing_handler: Arc<
    dyn Fn(SubscriberContext) -> BoxFuture<'static, Result<()>> + Send + Sync,
  > = Arc::new(move |context| {
    let lookup_interactor = LookupInteractor::new(Arc::clone(&context.redis_connection_pool));
    let event_publisher = EventPublisher::new(Arc::clone(&context.redis_connection_pool));
    Box::pin(async move {
      handle_file_processing_event(context, lookup_interactor, event_publisher).await
    })
  });

  vec![
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: Arc::clone(&settings),
      id: "lookup".to_string(),
      concurrency: Some(250),
      stream: Stream::Lookup,
      handle: Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&crawler_interactor);
        let lookup_interactor = LookupInteractor::new(Arc::clone(&context.redis_connection_pool));
        let album_read_model_repository = AlbumReadModelRepository {
          redis_connection_pool: Arc::clone(&context.redis_connection_pool),
        };
        let event_publisher = EventPublisher::new(Arc::clone(&context.redis_connection_pool));
        Box::pin(async move {
          handle_lookup_event(
            context,
            crawler_interactor,
            lookup_interactor,
            event_publisher,
            album_read_model_repository,
          )
          .await
        })
      }),
    },
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: Arc::clone(&settings),
      id: "lookup".to_string(),
      concurrency: Some(1),
      stream: Stream::File,
      handle: Arc::clone(&file_processing_handler),
    },
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: Arc::clone(&settings),
      id: "lookup".to_string(),
      concurrency: Some(1),
      stream: Stream::Parser,
      handle: Arc::clone(&file_processing_handler),
    },
  ]
}
