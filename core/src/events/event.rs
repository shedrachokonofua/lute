use serde::{Deserialize, Serialize};

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
