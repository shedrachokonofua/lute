use anyhow::Result;
use chrono::NaiveDate;
use clap::{arg, Parser};
use diesel::{
  upsert::excluded, Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use lute_postgres_connector::{
  client::lute::{
    event::Event, event_service_client::EventServiceClient, parsed_file_data::Data,
    EventStreamItem, EventStreamRequest,
  },
  models::*,
};
use std::{collections::HashMap, error::Error};
use tokio::sync::mpsc::unbounded_channel;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn run_migrations(
  connection: &mut PgConnection,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
  connection.run_pending_migrations(MIGRATIONS)?;
  Ok(())
}

pub fn establish_connection(database_url: &str) -> PgConnection {
  PgConnection::establish(&database_url)
    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

async fn store_lute_events(
  db_connection: &mut PgConnection,
  batch: &Vec<EventStreamItem>,
) -> Result<()> {
  use lute_postgres_connector::schema::lute_events::dsl::*;

  let new_records = batch
    .into_iter()
    .map(|item: &EventStreamItem| NewLuteEvent {
      entry_id: item.entry_id.clone(),
      stream_id: item.stream_id.clone(),
      payload: serde_json::to_value(&item.payload).unwrap(),
      event_timestamp: item.timestamp as i64,
    })
    .collect::<Vec<_>>();

  diesel::insert_into(lute_events)
    .values(&new_records)
    .on_conflict_do_nothing()
    .execute(db_connection)?;

  Ok(())
}

async fn store_albums(
  db_connection: &mut PgConnection,
  batch: &Vec<EventStreamItem>,
) -> Result<()> {
  let mut new_albums_map = HashMap::<String, LuteAlbum>::new();
  let mut new_artists_map = HashMap::<String, LuteArtist>::new();
  let mut new_album_artists_map = HashMap::<String, Vec<LuteAlbumArtist>>::new();
  let mut new_tracks_map = HashMap::<String, Vec<LuteTrack>>::new();
  let mut new_credits_map = HashMap::<String, Vec<LuteCredit>>::new();

  for item in batch {
    let payload = item.payload.as_ref();
    if payload.is_none() {
      continue;
    }
    let payload = payload.unwrap();
    let event = payload.event.as_ref();
    if event.is_none() {
      continue;
    }
    let event = event.unwrap().event.as_ref().unwrap();

    if let Event::FileParsed(file_parsed_event) = event {
      let parsed_file_data = file_parsed_event.data.as_ref();
      if parsed_file_data.is_none() {
        continue;
      }
      let data = parsed_file_data.unwrap().data.as_ref();
      if data.is_none() {
        continue;
      }
      let data = data.unwrap();

      if let Data::Album(parsed_album) = data {
        let new_album = LuteAlbum {
          file_name: file_parsed_event.file_name.clone(),
          name: parsed_album.name.clone(),
          rating: parsed_album.rating as f64,
          rating_count: parsed_album.rating_count as i32,
          primary_genres: parsed_album
            .primary_genres
            .iter()
            .map(|g| Some(g.clone()))
            .collect::<Vec<_>>(),
          secondary_genres: parsed_album
            .secondary_genres
            .iter()
            .map(|g| Some(g.clone()))
            .collect::<Vec<_>>(),
          descriptors: parsed_album
            .descriptors
            .iter()
            .map(|g| Some(g.clone()))
            .collect::<Vec<_>>(),
          languages: parsed_album
            .languages
            .iter()
            .map(|g| Some(g.clone()))
            .collect::<Vec<_>>(),
          release_date: parsed_album
            .release_date
            .clone()
            .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
        };
        let new_tracks = parsed_album
          .tracks
          .iter()
          .map(|track| LuteTrack {
            album_file_name: file_parsed_event.file_name.clone(),
            name: track.name.clone(),
            duration_seconds: track.duration_seconds.map(|d| d as i32),
            rating: track.rating.map(|r| r as f64),
            position: track.position.clone(),
          })
          .collect::<Vec<LuteTrack>>();
        let new_album_artists = parsed_album
          .artists
          .iter()
          .map(|artist| LuteAlbumArtist {
            album_file_name: file_parsed_event.file_name.clone(),
            artist_file_name: artist.file_name.clone(),
          })
          .collect::<Vec<LuteAlbumArtist>>();
        let new_credits = parsed_album
          .credits
          .iter()
          .map(|parsed_credit| LuteCredit {
            album_file_name: file_parsed_event.file_name.clone(),
            artist_file_name: parsed_credit.artist.as_ref().unwrap().file_name.clone(),
            roles: parsed_credit
              .roles
              .iter()
              .map(|r| Some(r.clone()))
              .collect::<Vec<_>>(),
          })
          .collect::<Vec<LuteCredit>>();
        let mut new_artists = parsed_album
          .artists
          .iter()
          .map(|artist| {
            (
              artist.file_name.clone(),
              LuteArtist {
                file_name: artist.file_name.clone(),
                name: artist.name.clone(),
              },
            )
          })
          .collect::<HashMap<String, LuteArtist>>();
        new_artists.extend(
          parsed_album
            .credits
            .iter()
            .map(|parsed_credit| {
              let artist = parsed_credit.artist.as_ref().unwrap();
              (
                artist.file_name.clone(),
                LuteArtist {
                  file_name: artist.file_name.clone(),
                  name: artist.name.clone(),
                },
              )
            })
            .collect::<HashMap<String, LuteArtist>>(),
        );

        new_albums_map.insert(file_parsed_event.file_name.clone(), new_album);
        new_artists_map.extend(new_artists);
        new_album_artists_map.insert(file_parsed_event.file_name.clone(), new_album_artists);
        new_tracks_map.insert(file_parsed_event.file_name.clone(), new_tracks);
        new_credits_map.insert(file_parsed_event.file_name.clone(), new_credits);
      }
    }
  }

  db_connection.transaction(|trx| {
    use lute_postgres_connector::schema::lute_albums::dsl::*;
    use lute_postgres_connector::schema::lute_artists::dsl::*;
    diesel::insert_into(lute_albums)
      .values(
        new_albums_map
          .into_iter()
          .map(|(_, v)| v)
          .collect::<Vec<_>>(),
      )
      .on_conflict(lute_postgres_connector::schema::lute_albums::dsl::file_name) // specify the unique column here
      .do_update()
      .set((
        lute_postgres_connector::schema::lute_albums::dsl::name.eq(excluded(
          lute_postgres_connector::schema::lute_albums::dsl::name,
        )),
        lute_postgres_connector::schema::lute_albums::dsl::rating.eq(excluded(
          lute_postgres_connector::schema::lute_albums::dsl::rating,
        )),
        rating_count.eq(excluded(rating_count)),
        primary_genres.eq(excluded(primary_genres)),
        secondary_genres.eq(excluded(secondary_genres)),
        descriptors.eq(excluded(descriptors)),
        release_date.eq(excluded(release_date)),
        languages.eq(excluded(languages)),
      ))
      .execute(trx)?;

    diesel::insert_into(lute_artists)
      .values(
        new_artists_map
          .clone()
          .into_iter()
          .map(|(_, v)| v)
          .collect::<Vec<_>>(),
      )
      .on_conflict(lute_postgres_connector::schema::lute_artists::dsl::file_name)
      .do_update()
      .set((
        lute_postgres_connector::schema::lute_artists::dsl::name.eq(excluded(
          lute_postgres_connector::schema::lute_artists::dsl::name,
        )),
      ))
      .execute(trx)?;

    diesel::result::QueryResult::Ok(())
  })?;
  db_connection.transaction(|trx| {
    use lute_postgres_connector::schema::lute_albums_artists::dsl::*;
    use lute_postgres_connector::schema::lute_credits::dsl::*;
    use lute_postgres_connector::schema::lute_tracks::dsl::*;
    diesel::insert_into(lute_albums_artists)
      .values(
        new_album_artists_map
          .clone()
          .values()
          .into_iter()
          .flat_map(|v| v)
          .collect::<Vec<_>>(),
      )
      .on_conflict_do_nothing()
      .execute(trx)?;

    diesel::insert_into(lute_tracks)
      .values(
        new_tracks_map
          .clone()
          .values()
          .into_iter()
          .flat_map(|v| v)
          .collect::<Vec<_>>(),
      )
      .on_conflict_do_nothing()
      .execute(trx)?;

    diesel::insert_into(lute_credits)
      .values(
        new_credits_map
          .clone()
          .values()
          .into_iter()
          .flat_map(|v| v)
          .collect::<Vec<_>>(),
      )
      .on_conflict_do_nothing()
      .execute(trx)?;

    diesel::result::QueryResult::Ok(())
  })?;
  Ok(())
}

async fn process_batch(
  db_connection: &mut PgConnection,
  batch: Vec<EventStreamItem>,
) -> Result<()> {
  store_lute_events(db_connection, &batch).await?;
  store_albums(db_connection, &batch).await?;
  Ok(())
}

fn event_stream_request(
  stream_id: &str,
  subscriber_id: &str,
  cursor: Option<String>,
) -> EventStreamRequest {
  EventStreamRequest {
    stream_id: stream_id.to_string(),
    subscriber_id: subscriber_id.to_string(),
    cursor,
    max_batch_size: Some(100),
  }
}

async fn subscribe(
  stream_id: String,
  subscriber_id: String,
  client: &mut EventServiceClient<tonic::transport::Channel>,
  db_connection: &mut PgConnection,
) -> Result<()> {
  let (cursor_sender, mut cursor_receiver) = unbounded_channel::<String>();
  let request_stream = async_stream::stream! {
    yield event_stream_request(&stream_id, &subscriber_id, None);

    while let Some(cursor) = cursor_receiver.recv().await {
      println!("Requesting batch with cursor: {}", cursor);
      yield event_stream_request(&stream_id, &subscriber_id,  Some(cursor));
    }
  };

  let response = client.stream(request_stream).await?;
  let mut event_stream = response.into_inner();

  while let Some(reply) = event_stream.message().await? {
    process_batch(db_connection, reply.items).await?;
    cursor_sender.send(reply.cursor)?;
  }

  Ok(())
}

#[derive(Parser, Debug)]
struct Args {
  #[arg(long, default_value = "grpc://localhost:22000")]
  lute_url: String,

  #[arg(long, default_value = "replication")]
  stream_id: String,

  #[arg(long)]
  subscriber_id: String,

  #[arg(long)]
  postgres_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let mut connection = establish_connection(&args.postgres_url);
  run_migrations(&mut connection).expect("Failed to run migrations");

  let mut client = EventServiceClient::connect(args.lute_url)
    .await
    .expect("Failed to connect to lute instance");

  subscribe(
    args.stream_id,
    args.subscriber_id,
    &mut client,
    &mut connection,
  )
  .await?;

  Ok(())
}
