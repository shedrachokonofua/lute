use anyhow::Result;
use clap::{arg, Parser};
use diesel::{Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use lute_postgres_connector::{
  client::lute::{
    event::Event, event_service_client::EventServiceClient, parsed_file_data::Data,
    EventStreamItem, EventStreamRequest, ParsedArtistReference,
  },
  models::{NewLuteArtist, NewLuteEvent},
};
use std::error::Error;
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
      id: item.entry_id.clone(),
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

fn to_new_artist(artist_ref: ParsedArtistReference) -> NewLuteArtist {
  NewLuteArtist {
    file_name: artist_ref.file_name,
    name: artist_ref.name,
  }
}

async fn store_artists(
  db_connection: &mut PgConnection,
  batch: &Vec<EventStreamItem>,
) -> Result<()> {
  use lute_postgres_connector::schema::lute_artists::dsl::*;

  let new_artists = batch
    .iter()
    .flat_map(|item| {
      let payload = item.payload.as_ref();
      if payload.is_none() {
        return vec![];
      }
      let payload = payload.unwrap();
      let event = payload.event.as_ref();
      if event.is_none() {
        return vec![];
      }
      let event = event.unwrap().event.as_ref().unwrap();

      match event {
        Event::FileParsed(file_parsed_event) => {
          let data = file_parsed_event
            .data
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap();

          match data {
            Data::Album(album) => {
              let mut refs = album
                .artists
                .clone()
                .into_iter()
                .map(to_new_artist)
                .collect::<Vec<_>>();

              refs.extend(
                album
                  .credits
                  .iter()
                  .filter_map(|credit| credit.artist.clone())
                  .map(to_new_artist),
              );
              refs
            }
            Data::Artist(artist) => vec![NewLuteArtist {
              file_name: file_parsed_event.file_name.clone(),
              name: artist.name.clone(),
            }],
            Data::Chart(chart) => (&chart.albums)
              .into_iter()
              .flat_map(|album| album.artists.clone())
              .map(to_new_artist)
              .collect::<Vec<_>>(),
            Data::AlbumSearchResult(search_result) => search_result
              .artists
              .clone()
              .into_iter()
              .map(to_new_artist)
              .collect::<Vec<_>>(),
          }
        }
        _ => vec![],
      }
    })
    .collect::<Vec<_>>();

  diesel::insert_into(lute_artists)
    .values(&new_artists)
    .on_conflict_do_nothing()
    .execute(db_connection)?;
  Ok(())
}

async fn process_batch(
  db_connection: &mut PgConnection,
  batch: Vec<EventStreamItem>,
) -> Result<()> {
  store_lute_events(db_connection, &batch).await?;
  store_artists(db_connection, &batch).await?;
  Ok(())
}

fn event_stream_request(subscriber_id: String, cursor: Option<String>) -> EventStreamRequest {
  EventStreamRequest {
    stream_id: "replication".to_string(),
    subscriber_id,
    cursor,
    max_batch_size: Some(100),
  }
}

async fn subscribe(
  subscriber_id: String,
  client: &mut EventServiceClient<tonic::transport::Channel>,
  db_connection: &mut PgConnection,
) -> Result<()> {
  let (cursor_sender, mut cursor_receiver) = unbounded_channel::<String>();
  let request_stream = async_stream::stream! {
    yield event_stream_request(subscriber_id.clone(), None);

    while let Some(cursor) = cursor_receiver.recv().await {
      println!("Requesting batch with cursor: {}", cursor);
      yield event_stream_request(subscriber_id.clone(), Some(cursor));
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
  #[arg(long)]
  subscriber_id: String,

  #[arg(long)]
  postgres_url: String,

  #[arg(long, default_value = "grpc://localhost:22000")]
  lute_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let mut connection = establish_connection(&args.postgres_url);
  run_migrations(&mut connection).expect("Failed to run migrations");

  let mut client = EventServiceClient::connect(args.lute_url)
    .await
    .expect("Failed to connect to lute instance");

  subscribe(args.subscriber_id.clone(), &mut client, &mut connection).await?;

  Ok(())
}
