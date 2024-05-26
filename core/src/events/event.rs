use crate::files::file_metadata::file_name::FileName;
use crate::lookup::album_search_lookup::AlbumSearchLookup;
use crate::parser::parsed_file_data::ParsedFileData;
use crate::profile::profile::ProfileId;
use crate::proto;
use anyhow::{anyhow, Result};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::EnumString;
use strum_macros;
use ulid::serde::ulid_as_u128;
use ulid::Ulid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum Event {
  FileSaved {
    #[serde(with = "ulid_as_u128")]
    file_id: Ulid,
    file_name: FileName,
  },
  FileDeleted {
    #[serde(with = "ulid_as_u128")]
    file_id: Ulid,
    file_name: FileName,
  },
  FileParsed {
    #[serde(with = "ulid_as_u128")]
    file_id: Ulid,
    file_name: FileName,
    data: ParsedFileData,
  },
  FileParseFailed {
    #[serde(with = "ulid_as_u128")]
    file_id: Ulid,
    file_name: FileName,
    error: String,
  },
  ProfileAlbumAdded {
    profile_id: ProfileId,
    file_name: FileName,
    factor: u32,
  },
  LookupAlbumSearchUpdated {
    lookup: AlbumSearchLookup,
  },
}

impl From<Event> for proto::Event {
  fn from(val: Event) -> Self {
    proto::Event {
      event: Some(match val {
        Event::FileSaved { file_id, file_name } => {
          proto::event::Event::FileSaved(proto::FileSavedEvent {
            file_id: file_id.to_string(),
            file_name: file_name.to_string(),
          })
        }
        Event::FileDeleted { file_id, file_name } => {
          proto::event::Event::FileDeleted(proto::FileDeletedEvent {
            file_id: file_id.to_string(),
            file_name: file_name.to_string(),
          })
        }
        Event::FileParsed {
          file_id,
          file_name,
          data,
        } => proto::event::Event::FileParsed(proto::FileParsedEvent {
          file_id: file_id.to_string(),
          file_name: file_name.to_string(),
          data: Some(data.into()),
        }),
        Event::FileParseFailed {
          file_id,
          file_name,
          error,
        } => proto::event::Event::FileParseFailed(proto::FileParseFailedEvent {
          file_id: file_id.to_string(),
          file_name: file_name.to_string(),
          error,
        }),
        Event::ProfileAlbumAdded {
          profile_id,
          file_name,
          factor,
        } => proto::event::Event::ProfileAlbumAdded(proto::ProfileAlbumAddedEvent {
          profile_id: profile_id.to_string(),
          file_name: file_name.to_string(),
          factor,
        }),
        Event::LookupAlbumSearchUpdated { lookup } => {
          proto::event::Event::LookupAlbumSearchUpdated(proto::LookupAlbumSearchUpdatedEvent {
            lookup: Some(lookup.into()),
          })
        }
      }),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
pub struct EventPayload {
  pub event: Event,
  #[builder(setter(into), default)]
  pub correlation_id: Option<String>,
  #[builder(setter(into), default)]
  pub causation_id: Option<String>,
  #[builder(setter(into), default)]
  pub metadata: Option<HashMap<String, String>>,
}

impl From<EventPayload> for proto::EventPayload {
  fn from(val: EventPayload) -> Self {
    proto::EventPayload {
      event: Some(val.event.into()),
      correlation_id: val.correlation_id,
      metadata: val.metadata.unwrap_or_default(),
    }
  }
}

impl EventPayload {
  pub fn from_event(event: Event) -> Self {
    EventPayloadBuilder::default().event(event).build().unwrap()
  }
}

impl From<EventPayload> for HashMap<String, String> {
  fn from(val: EventPayload) -> Self {
    let mut result = HashMap::new();
    result.insert(
      "event".to_string(),
      serde_json::to_string(&val.event).unwrap(),
    );
    result.insert(
      "metadata".to_string(),
      serde_json::to_string(&val.metadata.unwrap_or_default()).unwrap(),
    );
    if let Some(correlation_id) = val.correlation_id {
      result.insert("correlation_id".to_string(), correlation_id);
    }
    result
  }
}

impl TryFrom<&HashMap<String, String>> for EventPayload {
  type Error = anyhow::Error;

  fn try_from(value: &HashMap<String, String>) -> Result<Self> {
    let event = serde_json::from_str::<Event>(
      value
        .get("event")
        .ok_or(anyhow!("event not found in payload"))?,
    )?;
    let correlation_id = value.get("correlation_id").map(|value| value.to_string());
    let causation_id = value.get("causation_id").map(|value| value.to_string());
    let metadata = value
      .get("metadata")
      .map(|value| serde_json::from_str::<HashMap<String, String>>(value).unwrap());

    Ok(
      EventPayloadBuilder::default()
        .event(event)
        .correlation_id(correlation_id)
        .causation_id(causation_id)
        .metadata(metadata)
        .build()?,
    )
  }
}

#[derive(Clone, Debug, PartialEq, Eq, strum_macros::Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum Topic {
  File,
  Parser,
  Profile,
  Lookup,
  Global,
}
