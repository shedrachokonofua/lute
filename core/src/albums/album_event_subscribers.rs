use super::{
  album_interactor::AlbumInteractor,
  album_read_model::{
    AlbumReadModel, AlbumReadModelArtist, AlbumReadModelCredit, AlbumReadModelTrack,
  },
  album_repository::AlbumRepository,
  album_search_index::{AlbumEmbedding, AlbumSearchIndex},
  embedding_provider::{provider::AlbumEmbeddingProvider, AlbumEmbeddingProvidersInteractor},
  redis_album_search_index::RedisAlbumSearchIndex,
  sqlite_album_repository::SqliteAlbumRepository,
};
use crate::{
  crawler::{
    crawler_interactor::CrawlerInteractor,
    priority_queue::{Priority, QueuePushParameters},
  },
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, EventSubscriberBuilder, SubscriberContext},
  },
  files::file_metadata::page_type::PageType,
  helpers::key_value_store::KeyValueStore,
  parser::parsed_file_data::{ParsedArtistReference, ParsedCredit, ParsedFileData, ParsedTrack},
  settings::Settings,
  sqlite::SqliteConnection,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

impl From<&ParsedTrack> for AlbumReadModelTrack {
  fn from(parsed_track: &ParsedTrack) -> Self {
    Self {
      name: parsed_track.name.clone(),
      duration_seconds: parsed_track.duration_seconds,
      rating: parsed_track.rating,
      position: parsed_track.position.clone(),
    }
  }
}

impl From<&ParsedArtistReference> for AlbumReadModelArtist {
  fn from(parsed_artist: &ParsedArtistReference) -> Self {
    Self {
      name: parsed_artist.name.clone(),
      file_name: parsed_artist.file_name.clone(),
    }
  }
}

impl From<&ParsedCredit> for AlbumReadModelCredit {
  fn from(parsed_credit: &ParsedCredit) -> Self {
    Self {
      artist: (&parsed_credit.artist).into(),
      roles: parsed_credit.roles.clone(),
    }
  }
}

struct AlbumSubscriberContext {
  album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync>,
  album_interactor: Arc<AlbumInteractor>,
}

impl From<&SubscriberContext> for AlbumSubscriberContext {
  fn from(context: &SubscriberContext) -> Self {
    let album_embedding_providers_interactor = Arc::new(AlbumEmbeddingProvidersInteractor::new(
      Arc::clone(&context.settings),
      Arc::clone(&context.kv),
    ));
    let album_search_index: Arc<dyn AlbumSearchIndex + Send + Sync> =
      Arc::new(RedisAlbumSearchIndex::new(
        Arc::clone(&context.redis_connection_pool),
        Arc::clone(&album_embedding_providers_interactor),
      ));
    let album_repository: Arc<dyn AlbumRepository + Send + Sync> = Arc::new(
      SqliteAlbumRepository::new(Arc::clone(&context.sqlite_connection)),
    );
    let album_interactor = Arc::new(AlbumInteractor::new(
      Arc::clone(&album_repository),
      Arc::clone(&album_search_index),
    ));
    Self {
      album_search_index: album_search_index,
      album_interactor: album_interactor,
    }
  }
}

async fn update_album_read_models(context: SubscriberContext) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Album(parsed_album),
  } = &context.payload.event
  {
    let album_read_model = AlbumReadModel::from_parsed_album(&file_name, parsed_album.clone());
    AlbumSubscriberContext::from(&context)
      .album_interactor
      .put(album_read_model)
      .await?;
  }
  Ok(())
}

async fn delete_album_read_models(context: SubscriberContext) -> Result<()> {
  if let Event::FileDeleted { file_name, .. } = &context.payload.event {
    AlbumSubscriberContext::from(&context)
      .album_interactor
      .delete(file_name)
      .await?;
  }
  Ok(())
}

fn get_crawl_priority(correlation_id: Option<String>) -> Priority {
  correlation_id
    .map(|cid| {
      if cid.starts_with("crawl_similar_albums:") {
        Priority::Low
      } else {
        Priority::Standard
      }
    })
    .unwrap_or(Priority::Standard)
}

async fn crawl_chart_albums(
  context: SubscriberContext,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Chart(albums),
  } = context.payload.event
  {
    let priority = get_crawl_priority(context.payload.correlation_id);
    for album in albums {
      crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: album.file_name,
          priority: Some(priority),
          correlation_id: Some(format!("crawl_chart_albums:{}", file_name.to_string())),
          ..Default::default()
        })
        .await?;
    }
  }
  Ok(())
}

async fn crawl_artist_albums(
  context: SubscriberContext,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Artist(parsed_artist),
  } = context.payload.event
  {
    let priority = get_crawl_priority(context.payload.correlation_id);
    for album in parsed_artist.albums {
      crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: album.file_name,
          priority: Some(priority),
          correlation_id: Some(format!("crawl_artist_albums:{}", file_name.to_string())),
          ..Default::default()
        })
        .await?;
    }
  }
  Ok(())
}

async fn update_album_embedding(
  provider: Arc<dyn AlbumEmbeddingProvider + Send + Sync + 'static>,
  context: SubscriberContext,
) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Album(parsed_album),
  } = &context.payload.event
  {
    let album_read_model = AlbumReadModel::from_parsed_album(file_name, parsed_album.clone());
    let embedding = provider.generate(&album_read_model).await?;
    AlbumSubscriberContext::from(&context)
      .album_search_index
      .put_embedding(&AlbumEmbedding {
        file_name: file_name.clone(),
        key: provider.name().to_string(),
        embedding,
      })
      .await?;
  }
  Ok(())
}

fn build_embedding_provider_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<SqliteConnection>,
  settings: Arc<Settings>,
) -> Result<Vec<EventSubscriber>> {
  let album_embedding_providers_interactor = AlbumEmbeddingProvidersInteractor::new(
    Arc::clone(&settings),
    Arc::new(KeyValueStore::new(Arc::clone(&sqlite_connection))),
  );
  let subscribers = album_embedding_providers_interactor
    .providers
    .into_iter()
    .filter_map(|provider| {
      EventSubscriberBuilder::default()
        .id(format!("update_album_embedding:{}", provider.name()))
        .stream(Stream::Parser)
        .batch_size(provider.concurrency())
        .redis_connection_pool(Arc::clone(&redis_connection_pool))
        .sqlite_connection(Arc::clone(&sqlite_connection))
        .settings(Arc::clone(&settings))
        .handle(Arc::new(move |context| {
          let provider = Arc::clone(&provider);
          Box::pin(async move { update_album_embedding(provider, context).await })
        }))
        .build()
        .ok()
    })
    .collect::<Vec<EventSubscriber>>();
  Ok(subscribers)
}

pub fn build_album_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  sqlite_connection: Arc<SqliteConnection>,
  settings: Arc<Settings>,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<Vec<EventSubscriber>> {
  let album_crawler_interactor = Arc::clone(&crawler_interactor);
  let artist_crawler_interactor = Arc::clone(&crawler_interactor);
  let mut subscribers = vec![
    EventSubscriberBuilder::default()
      .id("update_album_read_models")
      .stream(Stream::Parser)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .sqlite_connection(Arc::clone(&sqlite_connection))
      .settings(Arc::clone(&settings))
      .generate_ordered_processing_group_id(Arc::new(|row| match &row.payload.event {
        Event::FileParsed {
          data: ParsedFileData::Album(album),
          ..
        } => Some(album.ascii_name()), // Ensure potential duplicates are processed sequentially
        _ => None,
      }))
      .handle(Arc::new(|context| {
        Box::pin(async move { update_album_read_models(context).await })
      }))
      .build()?,
    EventSubscriberBuilder::default()
      .id("delete_album_read_models")
      .stream(Stream::File)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .sqlite_connection(Arc::clone(&sqlite_connection))
      .settings(Arc::clone(&settings))
      .generate_ordered_processing_group_id(Arc::new(|row| match &row.payload.event {
        Event::FileDeleted { file_name, .. } => match file_name.page_type() {
          PageType::Album => Some(file_name.to_string()),
          _ => None,
        },
        _ => None,
      }))
      .handle(Arc::new(|context| {
        Box::pin(async move { delete_album_read_models(context).await })
      }))
      .build()?,
    EventSubscriberBuilder::default()
      .id("crawl_chart_albums")
      .stream(Stream::Parser)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .sqlite_connection(Arc::clone(&sqlite_connection))
      .settings(Arc::clone(&settings))
      .handle(Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&artist_crawler_interactor);
        Box::pin(async move { crawl_chart_albums(context, crawler_interactor).await })
      }))
      .build()?,
    EventSubscriberBuilder::default()
      .id("crawl_artist_albums")
      .stream(Stream::Parser)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .sqlite_connection(Arc::clone(&sqlite_connection))
      .settings(Arc::clone(&settings))
      .handle(Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&album_crawler_interactor);
        Box::pin(async move { crawl_artist_albums(context, crawler_interactor).await })
      }))
      .build()?,
  ];
  subscribers.append(&mut build_embedding_provider_event_subscribers(
    Arc::clone(&redis_connection_pool),
    Arc::clone(&sqlite_connection),
    Arc::clone(&settings),
  )?);
  Ok(subscribers)
}
