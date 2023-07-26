use super::album_read_model_repository::{
  AlbumReadModel, AlbumReadModelArtist, AlbumReadModelRepository, AlbumReadModelTrack,
};
use crate::{
  crawler::{crawler_interactor::CrawlerInteractor, priority_queue::QueuePushParameters},
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, SubscriberContext},
  },
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{ParsedAlbum, ParsedArtistReference, ParsedFileData, ParsedTrack},
  settings::Settings,
};
use anyhow::Result;
use chrono::Datelike;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

impl AlbumReadModelTrack {
  pub fn from_parsed_track(parsed_track: &ParsedTrack) -> Self {
    Self {
      name: parsed_track.name.clone(),
      duration_seconds: parsed_track.duration_seconds,
      rating: parsed_track.rating,
      position: parsed_track.position.clone(),
    }
  }
}

impl AlbumReadModelArtist {
  pub fn from_parsed_artist(parsed_artist: &ParsedArtistReference) -> Self {
    Self {
      name: parsed_artist.name.clone(),
      file_name: parsed_artist.file_name.clone(),
    }
  }
}

impl AlbumReadModel {
  pub fn from_parsed_album(file_name: &FileName, parsed_album: ParsedAlbum) -> Self {
    Self {
      name: parsed_album.name.clone(),
      file_name: file_name.clone(),
      rating: parsed_album.rating,
      rating_count: parsed_album.rating_count,
      artists: parsed_album
        .artists
        .iter()
        .map(AlbumReadModelArtist::from_parsed_artist)
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
        .map(AlbumReadModelTrack::from_parsed_track)
        .collect::<Vec<AlbumReadModelTrack>>(),
      release_date: parsed_album.release_date,
      release_year: parsed_album.release_date.map(|date| date.year() as u32),
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
    file_name: _,
    data: ParsedFileData::Artist(parsed_artist),
  } = context.payload.event
  {
    for album in parsed_artist.albums {
      crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: album.file_name,
          ..Default::default()
        })
        .await?;
    }
  }
  Ok(())
}

pub fn build_album_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Settings,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Vec<EventSubscriber> {
  let album_crawler_interactor = Arc::clone(&crawler_interactor);
  let artist_crawler_interactor = Arc::clone(&crawler_interactor);
  vec![
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "update_album_read_models".to_string(),
      concurrency: Some(250),
      stream: Stream::Parser,
      handle: Arc::new(|context| Box::pin(async move { update_album_read_models(context).await })),
    },
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "crawl_chart_albums".to_string(),
      concurrency: Some(250),
      stream: Stream::Parser,
      handle: Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&artist_crawler_interactor);
        Box::pin(async move { crawl_chart_albums(context, crawler_interactor).await })
      }),
    },
    EventSubscriber {
      redis_connection_pool: Arc::clone(&redis_connection_pool),
      settings: settings.clone(),
      id: "crawl_artist_albums".to_string(),
      concurrency: Some(250),
      stream: Stream::Parser,
      handle: Arc::new(move |context| {
        let crawler_interactor = Arc::clone(&album_crawler_interactor);
        Box::pin(async move { crawl_artist_albums(context, crawler_interactor).await })
      }),
    },
  ]
}
