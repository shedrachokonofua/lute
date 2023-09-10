use crate::settings::Settings;

use super::event::{EventPayload, Stream};
use super::event_subscriber_repository::EventSubscriberRepository;
use anyhow::Result;
use derive_builder::Builder;
use futures::future::{join_all, BoxFuture};
use iter_tools::Itertools;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
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
  event_subscriber_repository: EventSubscriberRepository,
  #[builder(default = "self.generate_default_ordered_processing_group_id()")]
  generate_ordered_processing_group_id:
    Option<Arc<dyn Fn((&String, &EventPayload)) -> Option<String> + Send + Sync>>,
}

impl EventSubscriberBuilder {
  pub fn get_default_event_subscriber_repository(
    &self,
  ) -> Result<EventSubscriberRepository, String> {
    match &self.sqlite_connection {
      Some(sqlite_connection) => Ok(EventSubscriberRepository::new(Arc::clone(
        sqlite_connection,
      ))),
      None => Err("SQLite connection is required".to_string()),
    }
  }

  pub fn generate_default_ordered_processing_group_id(
    &self,
  ) -> Option<Arc<dyn Fn((&String, &EventPayload)) -> Option<String> + Send + Sync>> {
    None
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
      .get_events_after_cursor(&self.stream, &self.id, self.batch_size)
      .await?;
    debug!(
      stream = self.stream.tag(),
      subscriber_id = self.id,
      count = &event_list.events.len(),
      "Polled stream"
    );
    let tail_cursor = event_list.tail_cursor();

    let mut ordered_processing_groups = vec![];
    for (key, group) in &event_list
      .events
      .into_iter()
      .group_by(|(id, event_payload)| {
        self
          .generate_ordered_processing_group_id
          .as_ref()
          .and_then(|f| f((id, event_payload)))
          .unwrap_or(id.clone())
      })
    {
      ordered_processing_groups.push((key, group.collect::<Vec<_>>()));
    }

    join_all(
      ordered_processing_groups
        .into_iter()
        .map(|(group_id, group)| {
          let redis_pool = Arc::clone(&self.redis_connection_pool);
          let sqlite_pool = Arc::clone(&self.sqlite_connection);
          let settings = Arc::clone(&self.settings);
          let handle = self.handle.clone();
          let subscriber_id = self.id.clone();
          let stream_tag = self.stream.tag();
          debug!(
            stream = stream_tag,
            subscriber_id,
            group_id = group_id,
            count = group.len(),
            "Processing group"
          );
          tokio::spawn(async move {
            for (entry_id, payload) in group {
              debug!(
                stream = stream_tag,
                subscriber_id,
                entry_id = entry_id,
                "Processing event"
              );
              handle(SubscriberContext {
                redis_connection_pool: Arc::clone(&redis_pool),
                sqlite_connection: Arc::clone(&sqlite_pool),
                settings: Arc::clone(&settings),
                entry_id: entry_id.clone(),
                payload: payload.clone(),
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
              })?;
            }
            Ok::<(), anyhow::Error>(())
          })
        }),
    )
    .await;

    Ok(tail_cursor)
  }

  pub async fn sleep(&self) {
    sleep(Duration::from_secs(1)).await;
  }

  pub async fn run(&self) -> Result<()> {
    loop {
      match self.poll_stream().await {
        Ok(Some(tail_cursor)) => {
          self.set_cursor(&tail_cursor).await?;
        }
        Ok(None) => {
          self.sleep().await;
        }
        Err(error) => {
          error!("Error polling stream: {}", error);
          self.sleep().await;
        }
      }
    }
  }
}
