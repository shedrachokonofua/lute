use super::event::{EventPayload, Stream};
use super::event_subscriber_repository::{
  EventList, EventRow, EventSubscriberRepository, EventSubscriberStatus,
};
use crate::context::ApplicationContext;
use crate::helpers::ThreadSafeAsyncFn;
use crate::scheduler::job_name::JobName;
use crate::scheduler::scheduler::{JobParametersBuilder, Scheduler};
use anyhow::Result;
use chrono::{NaiveDateTime, TimeDelta, Utc};
use derive_builder::Builder;
use futures::future::join_all;
use iter_tools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{debug, error};
use ulid::Ulid;

#[derive(Serialize, Deserialize)]
pub struct ChangeEventSubscriberStatusJobPayload {
  subscriber_id: String,
  status: EventSubscriberStatus,
}

pub struct EventSubscriberInteractor {
  subscriber_id: String,
  event_subscriber_repository: EventSubscriberRepository,
  scheduler: Arc<Scheduler>,
}

impl EventSubscriberInteractor {
  pub fn new(
    subscriber_id: String,
    event_subscriber_repository: EventSubscriberRepository,
    scheduler: Arc<Scheduler>,
  ) -> Self {
    Self {
      subscriber_id,
      event_subscriber_repository,
      scheduler,
    }
  }

  pub async fn get_cursor(&self) -> Result<String> {
    self
      .event_subscriber_repository
      .get_cursor(&self.subscriber_id)
      .await
  }

  pub async fn set_cursor(&self, cursor: &str) -> Result<()> {
    self
      .event_subscriber_repository
      .set_cursor(&self.subscriber_id, cursor)
      .await
  }

  pub async fn delete_cursor(&self) -> Result<()> {
    self
      .event_subscriber_repository
      .delete_cursor(&self.subscriber_id)
      .await
  }

  pub async fn get_events_after_cursor(
    &self,
    streams: &Vec<Stream>,
    count: usize,
  ) -> Result<EventList> {
    self
      .event_subscriber_repository
      .get_events_after_cursor(streams, &self.subscriber_id, count)
      .await
  }

  pub async fn get_status(&self) -> Result<Option<EventSubscriberStatus>> {
    self
      .event_subscriber_repository
      .get_status(&self.subscriber_id)
      .await
  }

  pub async fn set_status(&self, status: EventSubscriberStatus) -> Result<()> {
    self
      .event_subscriber_repository
      .set_status(&self.subscriber_id, status)
      .await
  }

  pub async fn schedule_status_change(
    &self,
    status: EventSubscriberStatus,
    when: NaiveDateTime,
  ) -> Result<String> {
    let id = Ulid::new().to_string();
    self
      .scheduler
      .put(
        JobParametersBuilder::default()
          .id(id.to_string())
          .name(JobName::ChangeEventSubscriberStatus)
          .next_execution(when)
          .payload(Some(serde_json::to_vec(
            &ChangeEventSubscriberStatusJobPayload {
              status,
              subscriber_id: self.subscriber_id.clone(),
            },
          )?))
          .build()?,
      )
      .await?;
    Ok(id.to_string())
  }

  pub async fn pause_for(&self, duration: TimeDelta) -> Result<String> {
    self.set_status(EventSubscriberStatus::Paused).await?;
    self
      .schedule_status_change(
        EventSubscriberStatus::Running,
        Utc::now().naive_utc() + duration,
      )
      .await
  }
}

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
  pub handle: ThreadSafeAsyncFn<(
    EventData,
    Arc<ApplicationContext>,
    Arc<EventSubscriberInteractor>,
  )>,
  #[builder(setter(skip), default = "self.get_default_interactor()?")]
  interactor: Arc<EventSubscriberInteractor>,
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
  pub fn get_default_interactor(&self) -> Result<Arc<EventSubscriberInteractor>, String> {
    match (&self.app_context, &self.id) {
      (Some(app_context), Some(id)) => Ok(Arc::new(EventSubscriberInteractor::new(
        id.clone(),
        EventSubscriberRepository::new(Arc::clone(&app_context.sqlite_connection)),
        Arc::clone(&app_context.scheduler),
      ))),
      _ => Err("SQLite connection and ID are required".to_string()),
    }
  }

  pub fn generate_default_ordered_processing_group_id(
    &self,
  ) -> Option<Arc<dyn Fn(&EventRow) -> Option<String> + Send + Sync>> {
    None
  }
}

impl EventSubscriber {
  pub async fn poll(&self) -> Result<Option<String>> {
    let event_list = self
      .interactor
      .get_events_after_cursor(&self.streams, self.batch_size)
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
          let interactor = Arc::clone(&self.interactor);
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
              handle((
                EventData {
                  entry_id: entry_id.clone(),
                  payload: payload.clone(),
                  stream: row.stream.clone(),
                },
                Arc::clone(&app_context),
                Arc::clone(&interactor),
              ))
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
    sleep(Duration::from_secs(2)).await;
  }

  pub async fn run(&self) -> Result<()> {
    loop {
      if self
        .interactor
        .get_status()
        .await?
        .is_some_and(|s| s == EventSubscriberStatus::Running)
      {
        if let Ok(Some(tail_cursor)) = self.poll().await.inspect_err(|e| {
          error!(
            subscriber_id = self.id,
            error = e.to_string(),
            "Error polling stream"
          );
        }) {
          self.interactor.set_cursor(&tail_cursor).await?;
        }
      }
      self.sleep().await;
    }
  }
}
