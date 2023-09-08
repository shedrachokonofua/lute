use super::event::{EventPayload, Stream};
use anyhow::Result;
use async_trait::async_trait;

pub struct EventList {
  pub events: Vec<(String, EventPayload)>,
}

impl EventList {
  pub fn tail_cursor(&self) -> Option<String> {
    self.events.last().map(|(id, _)| id.to_string())
  }
}
#[async_trait]
pub trait EventSubscriberRepository {
  async fn get_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<String>;
  async fn set_cursor(&self, stream: &Stream, subscriber_id: &str, cursor: &str) -> Result<()>;
  async fn delete_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<()>;
  async fn get_events_after_cursor(
    &self,
    stream: &Stream,
    subscriber_id: &str,
    count: usize,
    block: Option<u64>,
  ) -> Result<EventList>;
}
