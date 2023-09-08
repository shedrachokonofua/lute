use super::{
  event::{EventPayload, Stream},
  event_subscriber_repository::{EventList, EventSubscriberRepository},
};
use anyhow::Result;
use async_trait::async_trait;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{GenericCommands, StreamCommands, StreamEntry, StringCommands, XReadOptions},
};
use std::sync::Arc;

fn get_cursor_key(stream: &Stream, subscriber_id: &str) -> String {
  stream.redis_cursor_key(subscriber_id)
}

#[derive(Clone)]
pub struct RedisEventSubscriberRepository {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl RedisEventSubscriberRepository {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool,
    }
  }
}

#[async_trait]
impl EventSubscriberRepository for RedisEventSubscriberRepository {
  async fn get_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<String> {
    let cursor: Option<String> = self
      .redis_connection_pool
      .get()
      .await?
      .get(get_cursor_key(stream, subscriber_id))
      .await?;
    Ok(cursor.unwrap_or("0".to_string()))
  }

  async fn set_cursor(&self, stream: &Stream, subscriber_id: &str, cursor: &str) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .set(get_cursor_key(stream, subscriber_id), cursor)
      .await?;
    Ok(())
  }

  async fn delete_cursor(&self, stream: &Stream, subscriber_id: &str) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .del(get_cursor_key(stream, subscriber_id))
      .await?;
    Ok(())
  }

  async fn get_events_after_cursor(
    &self,
    stream: &Stream,
    subscriber_id: &str,
    count: usize,
    block: Option<u64>,
  ) -> Result<EventList> {
    let cursor = self.get_cursor(stream, subscriber_id).await?;
    let mut read_options = XReadOptions::default().count(count);
    if let Some(block) = block {
      read_options = read_options.block(block);
    }
    let results: Vec<(String, Vec<StreamEntry<String>>)> = self
      .redis_connection_pool
      .get()
      .await?
      .xread(read_options, [&stream.redis_key()], &cursor)
      .await?;
    if results.is_empty() {
      return Ok(EventList { events: vec![] });
    }
    let (_, items) = results.get(0).expect("no stream results");
    Ok(EventList {
      events: items
        .iter()
        .map(|item| {
          let id = item.stream_id.clone();
          let payload = EventPayload::try_from(&item.items).unwrap();
          (id, payload)
        })
        .collect(),
    })
  }
}
