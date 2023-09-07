use super::album_read_model_repository::{
  AlbumReadModel, AlbumReadModelArtist, AlbumReadModelCredit, AlbumReadModelRepository,
  AlbumReadModelTrack,
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
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{
    ParsedAlbum, ParsedArtistReference, ParsedCredit, ParsedFileData, ParsedTrack,
  },
  settings::Settings,
};
use anyhow::Result;
use chrono::Datelike;
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

impl AlbumReadModel {
  pub fn from_parsed_album(file_name: &FileName, parsed_album: ParsedAlbum) -> Self {
    let credit_tags = &parsed_album
      .credits
      .iter()
      .flat_map(|credit| {
        credit.roles.iter().map(|role| {
          format!(
            "{}:{}",
            credit.artist.file_name.to_string(),
            role.to_lowercase().replace(" ", "_")
          )
        })
      })
      .collect::<Vec<String>>();

    Self {
      name: parsed_album.name.clone(),
      file_name: file_name.clone(),
      rating: parsed_album.rating,
      rating_count: parsed_album.rating_count,
      artists: parsed_album
        .artists
        .iter()
        .map(AlbumReadModelArtist::from)
        .collect::<Vec<AlbumReadModelArtist>>(),
      artist_count: parsed_album.artists.len() as u32,
      primary_genres: parsed_album.primary_genres.clone(),
      primary_genre_count: parsed_album.primary_genres.len() as u32,
      secondary_genres: parsed_album.secondary_genres.clone(),
      secondary_genre_count: parsed_album.secondary_genres.len() as u32,
      descriptors: parsed_album.descriptors.clone(),
      descriptor_count: parsed_album.descriptors.len() as u32,
      tracks: parsed_album
        .tracks
        .iter()
        .map(AlbumReadModelTrack::from)
        .collect::<Vec<AlbumReadModelTrack>>(),
      release_date: parsed_album.release_date,
      release_year: parsed_album.release_date.map(|date| date.year() as u32),
      languages: parsed_album.languages.clone(),
      language_count: parsed_album.languages.len() as u32,
      credits: parsed_album
        .credits
        .iter()
        .map(AlbumReadModelCredit::from)
        .collect::<Vec<AlbumReadModelCredit>>(),
      credit_tag_count: credit_tags.len() as u32,
      credit_tags: credit_tags.clone(),
    }
  }
}

async fn update_album_read_models(context: SubscriberContext) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data: ParsedFileData::Album(parsed_album),
  } = context.payload.event
  {
    let album_read_model_repository = AlbumReadModelRepository {
      redis_connection_pool: Arc::clone(&context.redis_connection_pool),
    };
    let album_read_model = AlbumReadModel::from_parsed_album(&file_name, parsed_album);
    album_read_model_repository.put(album_read_model).await?;
  }
  Ok(())
}

async fn delete_album_read_models(context: SubscriberContext) -> Result<()> {
  if let Event::FileDeleted { file_name, .. } = context.payload.event {
    let album_read_model_repository = AlbumReadModelRepository {
      redis_connection_pool: Arc::clone(&context.redis_connection_pool),
    };
    album_read_model_repository.delete(&file_name).await?;
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

pub fn build_album_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Arc<Settings>,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<Vec<EventSubscriber>> {
  let album_crawler_interactor = Arc::clone(&crawler_interactor);
  let artist_crawler_interactor = Arc::clone(&crawler_interactor);
  Ok(vec![
    EventSubscriberBuilder::default()
      .id("update_album_read_models".to_string())
      .stream(Stream::Parser)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .settings(Arc::clone(&settings))
      .handle(Arc::new(|context| {
        Box::pin(async move { update_album_read_models(context).await })
      }))
      .build()?,
    EventSubscriberBuilder::default()
      .id("delete_album_read_models".to_string())
      .stream(Stream::File)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .settings(Arc::clone(&settings))
      .handle(Arc::new(|context| {
        Box::pin(async move { delete_album_read_models(context).await })
      }))
      .build()?,
    EventSubscriberBuilder::default()
      .id("crawl_chart_albums".to_string())
      .stream(Stream::Parser)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .settings(Arc::clone(&settings))
      .handle(Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&artist_crawler_interactor);
        Box::pin(async move { crawl_chart_albums(context, crawler_interactor).await })
      }))
      .build()?,
    EventSubscriberBuilder::default()
      .id("crawl_artist_albums".to_string())
      .stream(Stream::Parser)
      .batch_size(250)
      .redis_connection_pool(Arc::clone(&redis_connection_pool))
      .settings(Arc::clone(&settings))
      .handle(Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&album_crawler_interactor);
        Box::pin(async move { crawl_artist_albums(context, crawler_interactor).await })
      }))
      .build()?,
  ])
}
