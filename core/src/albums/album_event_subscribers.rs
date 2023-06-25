use super::album_read_model_repository::{
  AlbumReadModel, AlbumReadModelArtist, AlbumReadModelRepository, AlbumReadModelTrack,
};
use crate::{
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, SubscriberContext},
  },
  files::file_metadata::file_name::FileName,
  parser::parsed_file_data::{ParsedAlbum, ParsedArtistReference, ParsedFileData, ParsedTrack},
  settings::Settings,
};
use anyhow::Result;
use r2d2::Pool;
use redis::Client;
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
        .map(|artist| AlbumReadModelArtist::from_parsed_artist(artist))
        .collect::<Vec<AlbumReadModelArtist>>(),
      primary_genres: parsed_album.primary_genres.clone(),
      secondary_genres: parsed_album.secondary_genres.clone(),
      descriptors: parsed_album.descriptors.clone(),
      tracks: parsed_album
        .tracks
        .iter()
        .map(|track| AlbumReadModelTrack::from_parsed_track(track))
        .collect::<Vec<AlbumReadModelTrack>>(),
      release_date: parsed_album.release_date,
    }
  }
}

fn update_album_read_models(context: SubscriberContext) -> Result<()> {
  if let Event::FileParsed {
    file_id: _,
    file_name,
    data,
  } = context.payload.event
  {
    if let ParsedFileData::Album(parsed_album) = data {
      let album_read_model_repository = AlbumReadModelRepository {
        redis_connection_pool: Arc::clone(&context.redis_connection_pool),
      };
      let album_read_model = AlbumReadModel::from_parsed_album(&file_name, parsed_album);
      album_read_model_repository.put(album_read_model)?;
    }
  }
  Ok(())
}

pub fn build_album_event_subscribers(
  redis_connection_pool: Arc<Pool<Client>>,
  settings: Settings,
) -> Vec<EventSubscriber> {
  vec![EventSubscriber {
    redis_connection_pool,
    settings,
    id: "update_album_read_models".to_string(),
    concurrency: Some(250),
    stream: Stream::Parser,
    handle: Arc::new(|context| Box::pin(async move { update_album_read_models(context) })),
  }]
}
