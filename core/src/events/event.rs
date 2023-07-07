use crate::files::file_metadata::file_name::FileName;
use crate::lookup::album_search_lookup::AlbumSearchLookup;
use crate::parser::parsed_file_data::ParsedFileData;
use crate::profile::profile::ProfileId;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ulid::serde::ulid_as_u128;
use ulid::Ulid;

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Event {
  FileSaved {
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
  LookupAlbumSearchStatusChanged {
    lookup: AlbumSearchLookup,
  },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventPayload {
  pub event: Event,
  pub correlation_id: Option<String>,
  pub metadata: Option<HashMap<String, String>>,
}

impl EventPayload {
  pub fn from_event(event: Event) -> Self {
    EventPayload {
      event,
      correlation_id: None,
      metadata: None,
    }
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
      serde_json::to_string(&val.metadata.unwrap_or(HashMap::new())).unwrap(),
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
    let metadata = value
      .get("metadata")
      .map(|value| serde_json::from_str::<HashMap<String, String>>(value).unwrap());
    Ok(EventPayload {
      event,
      correlation_id,
      metadata,
    })
  }
}

pub enum Stream {
  File,
  Parser,
  Profile,
  Lookup,
}

impl Stream {
  pub fn tag(&self) -> String {
    match &self {
      Stream::File => "file".to_string(),
      Stream::Profile => "profile".to_string(),
      Stream::Parser => "parser".to_string(),
      Stream::Lookup => "lookup".to_string(),
    }
  }

  pub fn redis_key(&self) -> String {
    format!("event:stream:{}", &self.tag())
  }

  pub fn redis_cursor_key(&self, subscriber_id: &str) -> String {
    format!("event:stream:{}:cursor:{}", &self.tag(), subscriber_id)
  }
}
