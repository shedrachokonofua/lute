use anyhow::Result;
use clap::{arg, Parser};
use diesel::{Connection, PgConnection, RunQueryDsl};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use lute_postgres_connector::{
  client::lute::{event_service_client::EventServiceClient, EventStreamItem, EventStreamRequest},
  models::NewLuteEvent,
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

#[derive(Parser, Debug)]
struct Args {
  #[arg(long)]
  subscriber_id: String,

  #[arg(long)]
  postgres_url: String,

  #[arg(long, default_value = "grpc://localhost:22000")]
  lute_url: String,
}

async fn process_batch(
  db_connection: &mut PgConnection,
  batch: Vec<EventStreamItem>,
) -> Result<()> {
  use lute_postgres_connector::schema::lute_events::dsl::*;

  let new_records = batch
    .into_iter()
    .map(|item: EventStreamItem| NewLuteEvent {
      id: item.entry_id,
      stream_id: item.stream_id,
      payload: serde_json::to_value(item.payload).unwrap(),
      event_timestamp: item.timestamp as i64,
    })
    .collect::<Vec<_>>();

  diesel::insert_into(lute_events)
    .values(&new_records)
    .on_conflict_do_nothing()
    .execute(db_connection)?;

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
