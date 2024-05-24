use super::parsed_file_data::ParsedFileData;
use crate::{
  context::ApplicationContext,
  events::event::{Event, EventPayloadBuilder, Topic},
  files::file_metadata::{file_name::FileName, page_type::PageType},
  parser::{
    album::parse_album, album_search_result::parse_album_search_result, artist::parse_artist,
    chart::parse_chart,
  },
};
use anyhow::Result;
use std::sync::Arc;
use tracing::{info, instrument, warn};
use ulid::Ulid;

#[instrument(skip(app_context))]
pub async fn parse_file_on_store(
  app_context: Arc<ApplicationContext>,
  file_id: Ulid,
  file_name: FileName,
  correlation_id: Option<String>,
) -> Result<ParsedFileData> {
  let file_content = app_context
    .file_interactor
    .get_file_content(&file_name)
    .await?;

  let parse_result: Result<ParsedFileData> = match file_name.page_type() {
    PageType::Chart => parse_chart(&file_content).map(ParsedFileData::Chart),
    PageType::Album => parse_album(&file_content).map(ParsedFileData::Album),
    PageType::Artist => parse_artist(&file_content).map(ParsedFileData::Artist),
    PageType::AlbumSearchResult => {
      parse_album_search_result(&file_content).map(ParsedFileData::AlbumSearchResult)
    }
  };

  let event = match &parse_result {
    Ok(file_data) => {
      info!(
        file_id = file_id.to_string(),
        file_name = file_name.to_string(),
        page_type = file_name.page_type().to_string(),
        "File parsed"
      );

      Event::FileParsed {
        file_id,
        file_name: file_name.clone(),
        data: file_data.clone(),
      }
    }
    Err(error) => {
      warn!(
        file_id = file_id.to_string(),
        file_name = file_name.to_string(),
        page_type = file_name.page_type().to_string(),
        error = error.to_string(),
        "File parse failed"
      );

      Event::FileParseFailed {
        file_id,
        file_name: file_name.clone(),
        error: error.to_string(),
      }
    }
  };

  app_context
    .event_publisher
    .publish(
      Topic::Parser,
      EventPayloadBuilder::default()
        .event(event)
        .correlation_id(correlation_id)
        .build()?,
    )
    .await?;

  parse_result
}
