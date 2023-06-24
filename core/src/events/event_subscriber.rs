use crate::settings::Settings;

use super::event::{EventPayload, Stream};
use anyhow::Result;
use futures::future::{join_all, BoxFuture};
use r2d2::Pool;
use redis::{
  streams::{StreamReadOptions, StreamReadReply},
  Client, Commands,
};
use std::thread;
use std::{sync::Arc, time::Duration};
use tracing::debug;

pub struct SubscriberContext {
  pub entry_id: String,
  pub redis_connection_pool: Arc<Pool<Client>>,
  pub settings: Settings,
  pub payload: EventPayload,
}

pub struct EventSubscriber {
  pub concurrency: Option<usize>,
  pub redis_connection_pool: Arc<Pool<Client>>,
  pub settings: Settings,
  pub id: String,
  pub stream: Stream,
  pub handle: Arc<dyn Fn(SubscriberContext) -> BoxFuture<'static, Result<()>> + Send + Sync>,
}

impl EventSubscriber {
  pub fn get_cursor_key(&self) -> String {
    self.stream.redis_cursor_key(&self.id)
  }

  pub fn get_cursor(&self) -> Result<String> {
    let cursor: Option<String> = self
      .redis_connection_pool
      .get()?
      .get(self.get_cursor_key())?;
    Ok(cursor.unwrap_or("0".to_string()))
  }

  pub fn set_cursor(&self, cursor: &str) -> Result<()> {
    self
      .redis_connection_pool
      .get()?
      .set(self.get_cursor_key(), cursor)?;
    Ok(())
  }

  pub fn delete_cursor(&self) -> Result<()> {
    self
      .redis_connection_pool
      .get()?
      .del(self.get_cursor_key())?;
    Ok(())
  }

  pub async fn poll_stream(&self) -> Result<()> {
    let cursor = self.get_cursor()?;
    let reply: StreamReadReply = self.redis_connection_pool.get()?.xread_options(
      &[&self.stream.redis_key()],
      &[&cursor],
      &StreamReadOptions::default()
        .count(self.concurrency.unwrap_or(10))
        .block(500),
    )?;
    if let Some(stream) = reply.keys.get(0) {
      let futures = stream
        .ids
        .iter()
        .map(|id| {
          let entry_id = id.id.clone();
          let payload = EventPayload::try_from(id.map.clone()).unwrap();
          let pool = self.redis_connection_pool.clone();
          let settings = self.settings.clone();
          let handle = self.handle.clone();

          debug!(
            stream = self.stream.tag(),
            subscriber_id = self.id,
            entry_id = entry_id,
            "Handling event"
          );

          tokio::spawn(async move {
            let context = SubscriberContext {
              redis_connection_pool: pool,
              entry_id,
              settings,
              payload,
            };
            handle(context).await
          })
        })
        .collect::<Vec<_>>();

      join_all(futures).await;

      let tail = stream.ids.last().unwrap().id.clone();
      self.set_cursor(&tail)?;
    }
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
          println!("Error polling stream: {}", error);
          thread::sleep(std::time::Duration::from_secs(1));
        }
      }
    }
  }
}
