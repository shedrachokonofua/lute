use crate::settings::Settings;

use super::event::{EventPayload, Stream};
use anyhow::Result;
use futures::future::{join_all, BoxFuture};
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{GenericCommands, StreamCommands, StreamEntry, StringCommands, XReadOptions},
};
use std::thread;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error};

pub struct SubscriberContext {
  pub entry_id: String,
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub settings: Arc<Settings>,
  pub payload: EventPayload,
}

pub struct EventSubscriber {
  pub concurrency: Option<usize>,
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub settings: Arc<Settings>,
  pub id: String,
  pub stream: Stream,
  pub handle: Arc<dyn Fn(SubscriberContext) -> BoxFuture<'static, Result<()>> + Send + Sync>,
}

impl EventSubscriber {
  pub fn get_cursor_key(&self) -> String {
    self.stream.redis_cursor_key(&self.id)
  }

  pub async fn get_cursor(&self) -> Result<String> {
    let cursor: Option<String> = self
      .redis_connection_pool
      .get()
      .await?
      .get(self.get_cursor_key())
      .await?;
    Ok(cursor.unwrap_or("0".to_string()))
  }

  pub async fn set_cursor(&self, cursor: &str) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .set(self.get_cursor_key(), cursor)
      .await?;
    Ok(())
  }

  pub async fn delete_cursor(&self) -> Result<()> {
    self
      .redis_connection_pool
      .get()
      .await?
      .del(self.get_cursor_key())
      .await?;
    Ok(())
  }

  pub async fn poll_stream(&self) -> Result<()> {
    let cursor = self.get_cursor().await?;
    let results: Vec<(String, Vec<StreamEntry<String>>)> = self
      .redis_connection_pool
      .get()
      .await?
      .xread(
        XReadOptions::default()
          .count(self.concurrency.unwrap_or(10))
          .block(2500),
        [&self.stream.redis_key()],
        &cursor,
      )
      .await?;
    debug!(
      stream = self.stream.tag(),
      subscriber_id = self.id,
      count = results.len(),
      "Polled stream"
    );
    let stream_items = results.first().map(|(_, items)| items.clone());
    if stream_items.is_none() {
      return Ok(());
    }
    let stream_items = stream_items.unwrap();

    let futures = stream_items.iter().map(|entry| {
      let entry_id = entry.stream_id.clone();
      let payload = EventPayload::try_from(&entry.items).unwrap();
      let pool = Arc::clone(&self.redis_connection_pool);
      let settings = Arc::clone(&self.settings);
      let handle = self.handle.clone();
      let subscriber_id = self.id.clone();
      let stream_tag = self.stream.tag();

      debug!(
        stream = stream_tag,
        subscriber_id,
        entry_id = entry_id,
        "Handling event"
      );

      tokio::spawn(async move {
        handle(SubscriberContext {
          redis_connection_pool: pool,
          entry_id: entry_id.clone(),
          settings,
          payload,
        })
        .await
        .map_err(|err| {
          error!(
            stream = stream_tag,
            subscriber_id,
            entry_id = entry_id,
            error = err.to_string(),
            "Error handling event"
          );
          err
        })
      })
    });
    join_all(futures).await;
    let tail = stream_items.last().unwrap().stream_id.clone();
    self.set_cursor(&tail).await?;

    Ok(())
  }

  pub fn sleep(&self) {
    thread::sleep(Duration::from_secs(5));
  }

  pub async fn run(&self) {
    loop {
      match self.poll_stream().await {
        Ok(_) => {}
        Err(error) => {
          error!("Error polling stream: {}", error);
          thread::sleep(std::time::Duration::from_secs(1));
        }
      }
    }
  }
}
