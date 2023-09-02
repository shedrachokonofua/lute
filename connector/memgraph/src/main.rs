use anyhow::Result;
use chrono::NaiveDate;
use clap::{arg, Parser};
use futures::future::join_all;
use lute_memgraph_connector::client::lute::{
  event::Event, event_service_client::EventServiceClient, parsed_file_data::Data, EventStreamItem,
  EventStreamRequest, ParsedAlbum,
};
use neo4rs::{query, ConfigBuilder, Graph};
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

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
  graph: Arc<Graph>,
  file_name: String,
  parsed_album: ParsedAlbum,
) -> Result<()> {
  //println!("Processing album: {}", &file_name);
  let mut album_query = query(
      "MERGE (a:Album {file_name: $file_name}) SET a.name = $name, a.rating = $rating, a.rating_count = $rating_count, a.release_date = $release_date;",
    )
    .param("name", parsed_album.name.clone())
    .param("rating", parsed_album.rating as f64)
    .param("rating_count", parsed_album.rating_count as i64)
    .param("file_name", file_name.to_string());
  if let Some(release_date) = &parsed_album.release_date {
    album_query = album_query.param(
      "release_date",
      NaiveDate::parse_from_str(&release_date, "%Y-%m-%d")?,
    );
  } else {
    album_query = album_query.param("release_date", "");
  }
  graph.run(album_query).await?;

  for artist in &parsed_album.artists {
    graph
      .run(
        query("MERGE (a:Artist {file_name: $file_name}) SET a.name = $name;")
          .param("file_name", artist.file_name.to_string())
          .param("name", artist.name.clone()),
      )
      .await?;
    graph
      .run(
        query(
          "MATCH (a:Album {file_name: $album_file_name}), (b:Artist {file_name: $artist_file_name}) MERGE (b)-[:CREATED]->(a);",
        )
        .param("album_file_name", file_name.to_string())
        .param("artist_file_name", artist.file_name.clone()),
      )
      .await?;
  }

  for credit in &parsed_album.credits {
    let artist = credit.artist.clone().unwrap();
    for role in &credit.roles {
      graph
        .run(
          query("MERGE (a:Artist {file_name: $file_name}) SET a.name = $name;")
            .param("file_name", artist.file_name.to_string())
            .param("name", artist.name.clone()),
        )
        .await?;
      graph
        .run(
          query(
            "MATCH (a:Album {file_name: $album_file_name}), (b:Artist {file_name: $artist_file_name}) MERGE (b)-[:CREDITED {role: $role}]->(a);",
          )
          .param("album_file_name", file_name.to_string())
          .param("artist_file_name", artist.file_name.clone())
          .param("role", role.clone()),
        )
        .await?;
    }
  }

  for genre in &parsed_album.primary_genres {
    graph
      .run(query("MERGE (a:Genre {name: $name});").param("name", genre.clone()))
      .await?;
    graph
      .run(
        query(
          "MATCH (a:Album {file_name: $album_file_name}), (b:Genre {name: $name}) MERGE (a)-[:HAS_PRIMARY_GENRE]->(b);",
        )
        .param("album_file_name", file_name.to_string())
        .param("name", genre.clone()),
      )
      .await?;
  }

  for genre in &parsed_album.secondary_genres {
    graph
      .run(query("MERGE (a:Genre {name: $name});").param("name", genre.clone()))
      .await?;
    graph
      .run(
        query(
          "MATCH (a:Album {file_name: $album_file_name}), (b:Genre {name: $name}) MERGE (a)-[:HAS_SECONDARY_GENRE]->(b);",
        )
        .param("album_file_name", file_name.to_string())
        .param("name", genre.clone()),
      )
      .await?;
  }

  for descriptor in &parsed_album.descriptors {
    graph
      .run(query("MERGE (a:Descriptor {name: $name});").param("name", descriptor.clone()))
      .await?;
    graph
      .run(
        query(
          "MATCH (a:Album {file_name: $album_file_name}), (b:Descriptor {name: $name}) MERGE (a)-[:HAS_DESCRIPTOR]->(b);",
        )
        .param("album_file_name", file_name.to_string())
        .param("name", descriptor.clone()),
      )
      .await?;
  }

  Ok(())
}

fn get_album(event_stream_item: EventStreamItem) -> Option<(String, ParsedAlbum)> {
  let payload = event_stream_item.payload.as_ref();
  if payload.is_none() {
    return None;
  }
  let payload = payload.unwrap();
  let event = payload.event.as_ref();
  if event.is_none() {
    return None;
  }
  let event = event.unwrap().event.as_ref().unwrap();

  if let Event::FileParsed(file_parsed_event) = event {
    let parsed_file_data = file_parsed_event.data.as_ref();
    if parsed_file_data.is_none() {
      return None;
    }
    let data = parsed_file_data.unwrap().data.as_ref();
    if data.is_none() {
      return None;
    }
    let data = data.unwrap();

    if let Data::Album(parsed_album) = data {
      println!("Found album: {}", &file_parsed_event.file_name);
      return Some((file_parsed_event.file_name.clone(), parsed_album.clone()));
    }
  }

  None
}

async fn process_batch(graph: Arc<Graph>, batch: Vec<EventStreamItem>) -> Result<()> {
  let mut futures = batch
    .iter()
    .filter_map(|event_stream_item| get_album(event_stream_item.clone()))
    .map(|(file_name, parsed_album)| {
      tokio::spawn(process_album(Arc::clone(&graph), file_name, parsed_album))
    })
    .collect::<Vec<_>>();
  for chunk in futures.chunks_mut(25) {
    println!("Starting chunk of {} futures", chunk.len());
    join_all(chunk).await;
  }
  Ok(())
}

async fn subscribe(
  stream_id: String,
  subscriber_id: String,
  client: &mut EventServiceClient<tonic::transport::Channel>,
  graph: Arc<Graph>,
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
    process_batch(Arc::clone(&graph), reply.items).await?;
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

  #[arg(long, default_value = "localhost:7687")]
  db_uri: String,

  #[arg(long)]
  db_name: String,

  #[arg(long, default_value = "")]
  db_user: String,

  #[arg(long, default_value = "")]
  db_password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let graph = Arc::new(
    Graph::connect(
      ConfigBuilder::default()
        .uri(args.db_uri)
        .db(args.db_name)
        .user(args.db_user)
        .password(args.db_password)
        .build()?,
    )
    .await?,
  );
  let mut client = EventServiceClient::connect(args.lute_url)
    .await
    .expect("Failed to connect to lute instance");
  subscribe(args.stream_id, args.subscriber_id, &mut client, graph).await?;

  Ok(())
}
