use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
  FileSaved(FileSaved),
}

#[derive(Serialize, Deserialize)]
pub struct FileSaved {
  pub id: String,
  pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct EventPayload {
  pub event: Event,
  pub correlation_id: Option<String>,
  pub metadata: Option<HashMap<String, String>>,
}

impl Into<EventPayload> for Event {
  fn into(self) -> EventPayload {
    EventPayload {
      event: self,
      correlation_id: None,
      metadata: None,
    }
  }
}

impl Into<Vec<(String, String)>> for EventPayload {
  fn into(self) -> Vec<(String, String)> {
    vec![
      (
        "event".to_string(),
        serde_json::to_string(&self.event).unwrap(),
      ),
      (
        "correlation_id".to_string(),
        serde_json::to_string(&self.correlation_id).unwrap(),
      ),
      (
        "metadata".to_string(),
        serde_json::to_string(&self.metadata.unwrap_or(HashMap::new())).unwrap(),
      ),
    ]
  }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum EventTag {
  FileSaved,
}

impl From<&Event> for EventTag {
  fn from(event: &Event) -> Self {
    match event {
      Event::FileSaved { .. } => EventTag::FileSaved,
    }
  }
}

impl ToString for EventTag {
  fn to_string(&self) -> String {
    match self {
      EventTag::FileSaved => "file.saved".to_string(),
    }
  }
}
