use super::parsed_file_data::ParsedFileData;
use crate::{
  events::{
    event::{Event, EventPayload, Stream},
    event_publisher::EventPublisher,
  },
  files::{
    file_content_store::FileContentStore,
    file_metadata::{file_name::FileName, page_type::PageType},
  },
  parser::{album::parse_album, artist::parse_artist, chart::parse_chart},
};
use anyhow::Result;
use ulid::Ulid;

pub async fn parse_file_on_store(
  file_content_store: FileContentStore,
  event_publisher: EventPublisher,
  file_id: Ulid,
  file_name: FileName,
) -> Result<ParsedFileData> {
  let file_content = file_content_store.get(&file_name).await?;
  println!(
    "Parsing file: {} {} {}",
    file_id,
    file_name.to_string(),
    file_name.page_type().to_string()
  );

  let parse_result: Result<ParsedFileData> = match file_name.page_type() {
    PageType::Chart => parse_chart(&file_content).map(|albums| ParsedFileData::Chart { albums }),
    PageType::Album => parse_album(&file_content).map(|album| ParsedFileData::Album(album)),
    PageType::Artist => parse_artist(&file_content).map(|artist| ParsedFileData::Artist(artist)),
    _ => Err(anyhow::anyhow!("Unsupported page type").into()),
  };

  let event = match &parse_result {
    Ok(file_data) => Event::FileParsed {
      file_id,
      file_name,
      data: file_data.clone(),
    },
    Err(error) => Event::FileParseFailed {
      file_id,
      file_name,
      error: error.to_string(),
    },
  };

  event_publisher.publish(
    Stream::Parser,
    EventPayload {
      event,
      correlation_id: None,
      metadata: None,
    },
  )?;

  parse_result
}
