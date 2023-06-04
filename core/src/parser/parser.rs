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
  parser::chart::parse_chart,
};
use anyhow::{Ok, Result};
use ulid::Ulid;

pub async fn parse_file_on_store(
  file_content_store: FileContentStore,
  event_publisher: EventPublisher,
  file_id: Ulid,
  file_name: FileName,
) -> Result<ParsedFileData> {
  let file_content = file_content_store.get(&file_name).await?;
  println!(
    "Parsing file: {} {}",
    file_name.to_string(),
    file_name.page_type().to_string()
  );

  let file_data = match file_name.page_type() {
    PageType::Chart => {
      let albums = parse_chart(&file_content)?;
      Ok(ParsedFileData::Chart { albums })
    }
    _ => Err(anyhow::anyhow!("Unsupported page type").into()),
  }?;

  event_publisher.publish(
    Stream::Parser,
    EventPayload {
      event: Event::FileParsed {
        file_id,
        file_name,
        data: file_data.clone(),
      },
      correlation_id: None,
      metadata: None,
    },
  )?;

  Ok(file_data)
}
