use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDate;
use clap::{arg, Parser};
use lute_memgraph_connector::client::lute::{
  event::Event, event_service_client::EventServiceClient, parsed_file_data::Data, EventStreamItem,
  EventStreamRequest, ParsedAlbum,
};
use rsmgclient::{ConnectParams, Connection, MgError, QueryParam, SSLMode, Value};
use tokio::sync::mpsc::unbounded_channel;

#[derive(Parser, Debug)]
struct Args {
  #[arg(long, default_value = "grpc://localhost:22000")]
  lute_url: String,

  #[arg(long, default_value = "replication")]
  stream_id: String,

  #[arg(long)]
  subscriber_id: String,

  #[arg(long, default_value = "localhost")]
  memgraph_host: String,

  #[arg(long, default_value = "7687")]
  memgraph_port: u16,
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

async fn process_album(
  db_connection: &mut Connection,
  file_name: String,
  parsed_album: &ParsedAlbum,
) -> Result<(), MgError> {
  println!("Processing album: {}", &file_name);
  db_connection.execute(
    "MERGE (a:Album {file_name: $file_name}) SET a.name = $name, a.rating = $rating, a.rating_count = $rating_count, a.release_date = $release_date;",
    Some(&HashMap::from([
      (
        "name".to_string(),
        QueryParam::String(parsed_album.name.clone()),
      ),
      (
        "file_name".to_string(),
        QueryParam::String(file_name.to_string()),
      ),
      (
        "rating".to_string(),
        QueryParam::Float(parsed_album.rating as f64),
      ),
      (
        "rating_count".to_string(),
        QueryParam::Int(parsed_album.rating_count as i64),
      ),
      (
        "release_date".to_string(),
        parsed_album.release_date
          .clone()
          .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
          .map(|d| QueryParam::Date(d))
          .unwrap_or(QueryParam::Null),
      ),
    ])),
  )?;
  db_connection.fetchall()?;

  for artist in &parsed_album.artists {
    db_connection.execute(
      "MERGE (a:Artist {file_name: $file_name}) SET a.name = $name;",
      Some(&HashMap::from([
        (
          "file_name".to_string(),
          QueryParam::String(artist.file_name.to_string()),
        ),
        ("name".to_string(), QueryParam::String(artist.name.clone())),
      ])),
    )?;
    db_connection.fetchall()?;
    db_connection.execute(
      "MATCH (a:Album {file_name: $album_file_name}), (b:Artist {file_name: $artist_file_name}) MERGE (b)-[:CREATED]->(a);",
      Some(&HashMap::from([
        (
          "album_file_name".to_string(),
          QueryParam::String(file_name.to_string()),
        ),
        (
          "artist_file_name".to_string(),
          QueryParam::String(artist.file_name.clone()),
        ),
      ])),
    )?;
    db_connection.fetchall()?;
  }

  for credit in &parsed_album.credits {
    let artist = credit.artist.clone().unwrap();
    for role in &credit.roles {
      db_connection.execute(
        "MERGE (a:Artist {file_name: $file_name}) SET a.name = $name;",
        Some(&HashMap::from([
          (
            "file_name".to_string(),
            QueryParam::String(artist.file_name.to_string()),
          ),
          ("name".to_string(), QueryParam::String(artist.name.clone())),
        ])),
      )?;
      db_connection.fetchall()?;
      db_connection.execute(
        "MATCH (a:Album {file_name: $album_file_name}), (b:Artist {file_name: $artist_file_name}) MERGE (b)-[:CREDITED {role: $role}]->(a);",
        Some(&HashMap::from([
          (
            "album_file_name".to_string(),
            QueryParam::String(file_name.to_string()),
          ),
          (
            "artist_file_name".to_string(),
            QueryParam::String(artist.file_name.clone()),
          ),
          ("role".to_string(), QueryParam::String(role.clone())),
        ])),
      )?;
      db_connection.fetchall()?;
    }
  }

  for genre in &parsed_album.primary_genres {
    db_connection.execute(
      "MERGE (a:Genre {name: $name});",
      Some(&HashMap::from([(
        "name".to_string(),
        QueryParam::String(genre.clone()),
      )])),
    )?;
    db_connection.fetchall()?;
    db_connection.execute(
      "MATCH (a:Album {file_name: $album_file_name}), (b:Genre {name: $name}) MERGE (a)-[:HAS_PRIMARY_GENRE]->(b);",
      Some(&HashMap::from([
        (
          "album_file_name".to_string(),
          QueryParam::String(file_name.to_string()),
        ),
        (
          "name".to_string(),
          QueryParam::String(genre.clone()),
        ),
      ])),
    )?;
    db_connection.fetchall()?;
  }

  for genre in &parsed_album.secondary_genres {
    db_connection.execute(
      "MERGE (a:Genre {name: $name});",
      Some(&HashMap::from([(
        "name".to_string(),
        QueryParam::String(genre.clone()),
      )])),
    )?;
    db_connection.fetchall()?;
    db_connection.execute(
      "MATCH (a:Album {file_name: $album_file_name}), (b:Genre {name: $name}) MERGE (a)-[:HAS_SECONDARY_GENRE]->(b);",
      Some(&HashMap::from([
        (
          "album_file_name".to_string(),
          QueryParam::String(file_name.to_string()),
        ),
        (
          "name".to_string(),
          QueryParam::String(genre.clone()),
        ),
      ])),
    )?;
    db_connection.fetchall()?;
  }

  for descriptor in &parsed_album.descriptors {
    db_connection.execute(
      "MERGE (a:Descriptor {name: $name});",
      Some(&HashMap::from([(
        "name".to_string(),
        QueryParam::String(descriptor.clone()),
      )])),
    )?;
    db_connection.fetchall()?;
    db_connection.execute(
      "MATCH (a:Album {file_name: $album_file_name}), (b:Descriptor {name: $name}) MERGE (a)-[:HAS_DESCRIPTOR]->(b);",
      Some(&HashMap::from([
        (
          "album_file_name".to_string(),
          QueryParam::String(file_name.to_string()),
        ),
        (
          "name".to_string(),
          QueryParam::String(descriptor.clone()),
        ),
      ])),
    )?;
    db_connection.fetchall()?;
  }

  db_connection.commit()?;
  Ok(())
}

async fn process_batch(db_connection: &mut Connection, batch: Vec<EventStreamItem>) -> Result<()> {
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
        process_album(
          db_connection,
          file_parsed_event.file_name.clone(),
          parsed_album,
        )
        .await
        .map_err(|e| {
          anyhow::anyhow!(
            "Failed to process album {}. Error: {}",
            file_parsed_event.file_name,
            &e.to_string()
          )
        })?;
      }
    }
  }
  Ok(())
}

async fn subscribe(
  stream_id: String,
  subscriber_id: String,
  client: &mut EventServiceClient<tonic::transport::Channel>,
  db_connection: &mut Connection,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let mut connection = Connection::connect(&ConnectParams {
    host: Some(args.memgraph_host.clone()),
    port: args.memgraph_port.clone(),
    sslmode: SSLMode::Disable,
    ..Default::default()
  })
  .map_err(|e| {
    anyhow::anyhow!(
      "Failed to connect to Memgraph at {}:{}. Error: {}",
      args.memgraph_host,
      args.memgraph_port,
      &e.to_string()
    )
  })?;
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
