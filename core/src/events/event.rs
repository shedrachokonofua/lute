use crate::files::file_metadata::file_name::FileName;
use crate::lookup::AlbumSearchLookup;
use crate::parser::parsed_file_data::ParsedFileData;
use crate::profile::profile::ProfileId;
use crate::proto;
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
  AlbumSaved {
    file_name: FileName,
  },
  CrawlEnqueued {
    file_name: FileName,
  },
  CrawlFailed {
    file_name: FileName,
    error: String,
  },
  ListSegmentSaved {
    file_name: FileName,
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
        Event::AlbumSaved { file_name } => {
          proto::event::Event::AlbumSaved(proto::AlbumSavedEvent {
            file_name: file_name.to_string(),
          })
        }
        Event::CrawlEnqueued { file_name } => {
          proto::event::Event::CrawlEnqueued(proto::CrawlEnqueuedEvent {
            file_name: file_name.to_string(),
          })
        }
        Event::CrawlFailed { file_name, error } => {
          proto::event::Event::CrawlFailed(proto::CrawlFailedEvent {
            file_name: file_name.to_string(),
            error,
          })
        }
        Event::ListSegmentSaved { file_name } => {
          proto::event::Event::ListSegmentSaved(proto::ListSegmentSavedEvent {
            file_name: file_name.to_string(),
          })
        }
      }),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Builder, Debug)]
pub struct EventPayload {
  pub event: Event,
  /**
   * Events are uniquely identified by their key per topic.
   */
  #[builder(setter(into))]
  pub key: String,
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

#[derive(Clone, Debug, PartialEq, Eq, strum_macros::Display, EnumString, Hash)]
#[strum(serialize_all = "kebab-case")]
pub enum Topic {
  File,
  Parser,
  Profile,
  Lookup,
  Album,
  All,
}
