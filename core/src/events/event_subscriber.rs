use super::event::{EventPayload, Stream};
use super::event_subscriber_repository::{EventRow, EventSubscriberRepository};
use crate::context::ApplicationContext;
use anyhow::Result;
use derive_builder::Builder;
use futures::future::{join_all, BoxFuture};
use iter_tools::Itertools;
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{debug, error};

pub struct EventData {
  pub entry_id: String,
  pub stream: Stream,
  pub payload: EventPayload,
}

#[derive(Builder)]
pub struct EventSubscriber {
  #[builder(default = "10")]
  pub batch_size: usize,
  pub app_context: Arc<ApplicationContext>,
  #[builder(setter(into))]
  pub id: String,
  #[builder(setter(each(name = "stream")))]
  pub streams: Vec<Stream>,
  pub handle:
    Arc<dyn Fn(EventData, Arc<ApplicationContext>) -> BoxFuture<'static, Result<()>> + Send + Sync>,
  #[builder(
    setter(skip),
    default = "self.get_default_event_subscriber_repository()?"
  )]
  event_subscriber_repository: EventSubscriberRepository,
  /**
   * A function that returns a processing group ID for the given event. Events with the same processing group ID will be processed in order.
   */
  #[builder(
    default = "self.generate_default_ordered_processing_group_id()",
    setter(strip_option)
  )]
  generate_ordered_processing_group_id:
    Option<Arc<dyn Fn(&EventRow) -> Option<String> + Send + Sync>>,
}

impl EventSubscriberBuilder {
  pub fn get_default_event_subscriber_repository(
    &self,
  ) -> Result<EventSubscriberRepository, String> {
    match &self.app_context {
      Some(app_context) => Ok(EventSubscriberRepository::new(Arc::clone(
        &app_context.sqlite_connection,
      ))),
      None => Err("SQLite connection is required".to_string()),
    }
  }

  pub fn generate_default_ordered_processing_group_id(
    &self,
  ) -> Option<Arc<dyn Fn(&EventRow) -> Option<String> + Send + Sync>> {
    None
  }
}

impl EventSubscriber {
  pub async fn get_cursor(&self) -> Result<String> {
    self.event_subscriber_repository.get_cursor(&self.id).await
  }

  pub async fn set_cursor(&self, cursor: &str) -> Result<()> {
    self
      .event_subscriber_repository
      .set_cursor(&self.id, cursor)
      .await
  }

  pub async fn delete_cursor(&self) -> Result<()> {
    self
      .event_subscriber_repository
      .delete_cursor(&self.id)
      .await
  }

  pub async fn poll(&self) -> Result<Option<String>> {
    let event_list = self
      .event_subscriber_repository
      .get_events_after_cursor(&self.streams, &self.id, self.batch_size)
      .await?;
    let stream_tags = self.streams.iter().map(|s| s.tag()).join(",");
    debug!(
      streams = stream_tags.as_str(),
      subscriber_id = self.id,
      count = &event_list.rows.len(),
      "Subscriber polled"
    );
    let tail_cursor = event_list.tail_cursor();

    let mut ordered_processing_groups: HashMap<String, Vec<EventRow>> = HashMap::new();
    for (key, group) in &event_list.rows.into_iter().group_by(|row| {
      self
        .generate_ordered_processing_group_id
        .as_ref()
        .and_then(|f| f(row))
        .unwrap_or(row.id.clone())
    }) {
      ordered_processing_groups
        .entry(key.clone())
        .or_default()
        .extend(group);
    }

    join_all(
      ordered_processing_groups
        .into_iter()
        .map(|(group_id, group)| {
          let app_context = Arc::clone(&self.app_context);
          let handle = self.handle.clone();
          let subscriber_id = self.id.clone();
          let stream_tags = stream_tags.clone();

          debug!(
            streams = stream_tags.as_str(),
            subscriber_id,
            group_id = group_id,
            count = group.len(),
            "Processing group"
          );
          tokio::spawn(async move {
            for row in group {
              let entry_id = row.id;
              let payload = row.payload;
              debug!(
                streams = stream_tags.as_str(),
                subscriber_id,
                entry_id = entry_id,
                event_kind = payload.event.kind().to_string(),
                correlation_id = payload.correlation_id.clone(),
                causation_id = payload.causation_id.clone(),
                "Processing event"
              );
              handle(
                EventData {
                  entry_id: entry_id.clone(),
                  payload: payload.clone(),
                  stream: row.stream.clone(),
                },
                Arc::clone(&app_context),
              )
              .await
              .map_err(|err| {
                error!(
                  stream = stream_tags.as_str(),
                  subscriber_id,
                  entry_id = entry_id,
                  error = err.to_string(),
                  correlation_id = payload.correlation_id,
                  causation_id = payload.causation_id,
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
      match self.poll().await {
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
