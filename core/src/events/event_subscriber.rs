use crate::settings::Settings;

use super::{
  event::{EventPayload, Stream},
  event_subscriber_repository::EventSubscriberRepository,
  sqlite_event_subscriber_repository::SqliteEventSubscriberRepository,
};
use anyhow::Result;
use derive_builder::Builder;
use futures::future::{join_all, BoxFuture};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::thread;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error};

pub struct SubscriberContext {
  pub entry_id: String,
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub sqlite_connection: Arc<tokio_rusqlite::Connection>,
  pub settings: Arc<Settings>,
  pub payload: EventPayload,
}

#[derive(Builder)]
pub struct EventSubscriber {
  #[builder(default = "10")]
  pub batch_size: usize,
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub sqlite_connection: Arc<tokio_rusqlite::Connection>,
  pub settings: Arc<Settings>,
  pub id: String,
  pub stream: Stream,
  pub handle: Arc<dyn Fn(SubscriberContext) -> BoxFuture<'static, Result<()>> + Send + Sync>,
  #[builder(
    setter(skip),
    default = "self.get_default_event_subscriber_repository()?"
  )]
  event_subscriber_repository: SqliteEventSubscriberRepository,
}

impl EventSubscriberBuilder {
  pub fn get_default_event_subscriber_repository(
    &self,
  ) -> Result<SqliteEventSubscriberRepository, String> {
    match &self.sqlite_connection {
      Some(sqlite_connection) => Ok(SqliteEventSubscriberRepository::new(Arc::clone(
        sqlite_connection,
      ))),
      None => Err("SQLite connection pool is required".to_string()),
    }
  }
}

impl EventSubscriber {
  pub async fn get_cursor(&self) -> Result<String> {
    self
      .event_subscriber_repository
      .get_cursor(&self.stream, &self.id)
      .await
  }

  pub async fn set_cursor(&self, cursor: &str) -> Result<()> {
    self
      .event_subscriber_repository
      .set_cursor(&self.stream, &self.id, cursor)
      .await
  }

  pub async fn delete_cursor(&self) -> Result<()> {
    self
      .event_subscriber_repository
      .delete_cursor(&self.stream, &self.id)
      .await
  }

  pub async fn poll_stream(&self) -> Result<Option<String>> {
    let event_list = self
      .event_subscriber_repository
      .get_events_after_cursor(&self.stream, &self.id, self.batch_size, Some(10000))
      .await?;
    debug!(
      stream = self.stream.tag(),
      subscriber_id = self.id,
      count = &event_list.events.len(),
      "Polled stream"
    );
    let tail_cursor = event_list.tail_cursor();
    let futures = event_list.events.into_iter().map(|(event_id, payload)| {
      let redis_pool = Arc::clone(&self.redis_connection_pool);
      let sqlite_pool = Arc::clone(&self.sqlite_connection);
      let settings = Arc::clone(&self.settings);
      let handle = self.handle.clone();
      let subscriber_id = self.id.clone();
      let stream_tag = self.stream.tag();

      debug!(
        stream = stream_tag,
        subscriber_id,
        event_id = event_id,
        "Handling event"
      );

      tokio::spawn(async move {
        handle(SubscriberContext {
          redis_connection_pool: redis_pool,
          sqlite_connection: sqlite_pool,
          entry_id: event_id.clone(),
          settings,
          payload: payload.clone(),
        })
        .await
        .map_err(|err| {
          error!(
            stream = stream_tag,
            subscriber_id,
            event_id = event_id,
            error = err.to_string(),
            "Error handling event"
          );
          err
        })
      })
    });
    join_all(futures).await;

    Ok(tail_cursor)
  }

  pub fn sleep(&self) {
    thread::sleep(Duration::from_secs(1));
  }

  pub async fn run(&self) -> Result<()> {
    loop {
      match self.poll_stream().await {
        Ok(Some(tail_cursor)) => {
          self.set_cursor(&tail_cursor).await?;
        }
        Ok(None) => {
          self.sleep();
        }
        Err(error) => {
          error!("Error polling stream: {}", error);
          self.sleep();
        }
      }
    }
  }
}
