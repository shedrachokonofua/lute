use super::event::{EventPayload, Topic};
use super::event_repository::{EventList, EventRepository, EventRow, EventSubscriberStatus};
use crate::context::ApplicationContext;
use crate::helpers::async_utils::ThreadSafeAsyncFn;
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
use tracing::{debug, error, info};
use ulid::Ulid;

#[derive(Serialize, Deserialize)]
pub struct ChangeEventSubscriberStatusJobPayload {
  subscriber_id: String,
  status: EventSubscriberStatus,
}

pub struct EventSubscriberInteractor {
  subscriber_id: String,
  event_repository: EventRepository,
  scheduler: Arc<Scheduler>,
}

impl EventSubscriberInteractor {
  pub fn new(
    subscriber_id: String,
    event_repository: EventRepository,
    scheduler: Arc<Scheduler>,
  ) -> Self {
    Self {
      subscriber_id,
      event_repository,
      scheduler,
    }
  }

  pub async fn get_cursor(&self) -> Result<String> {
    self.event_repository.get_cursor(&self.subscriber_id).await
  }

  pub async fn set_cursor(&self, cursor: &str) -> Result<()> {
    self
      .event_repository
      .set_cursor(&self.subscriber_id, cursor)
      .await
  }

  pub async fn delete_cursor(&self) -> Result<()> {
    self
      .event_repository
      .delete_cursor(&self.subscriber_id)
      .await
  }

  pub async fn get_events_after_cursor(
    &self,
    topics: &Vec<Topic>,
    count: usize,
  ) -> Result<EventList> {
    self
      .event_repository
      .get_events_after_cursor(topics, &self.subscriber_id, count)
      .await
  }

  pub async fn get_status(&self) -> Result<Option<EventSubscriberStatus>> {
    self
      .event_repository
      .get_subscriber_status(&self.subscriber_id)
      .await
  }

  pub async fn set_status(&self, status: EventSubscriberStatus) -> Result<()> {
    self
      .event_repository
      .set_subscriber_status(&self.subscriber_id, status)
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
          .payload(serde_json::to_vec(
            &ChangeEventSubscriberStatusJobPayload {
              status,
              subscriber_id: self.subscriber_id.clone(),
            },
          )?)
          .build()?,
      )
      .await?;
    Ok(id.to_string())
  }

  pub async fn pause_until(&self, until: NaiveDateTime) -> Result<String> {
    self.set_status(EventSubscriberStatus::Paused).await?;
    self
      .schedule_status_change(EventSubscriberStatus::Running, until)
      .await
  }

  pub async fn pause_for(&self, duration: TimeDelta) -> Result<String> {
    self.pause_until(Utc::now().naive_utc() + duration).await
  }
}

pub struct EventData {
  pub entry_id: String,
  pub topic: Topic,
  pub payload: EventPayload,
}

#[derive(Clone, Default)]
pub enum GroupingStrategy {
  /**
   * Every event in the batch will be processed in parallel.
   */
  #[default]
  Individual,
  Chunks(usize),
  GroupByKey(Arc<dyn Fn(&EventRow) -> String + Send + Sync>),
  GroupByCorrelationId,
  /**
   * All events in the batch will be processed in a single call.
   */
  All,
}

impl GroupingStrategy {
  pub fn group(&self, events: Vec<EventRow>) -> Vec<(String, Vec<EventRow>)> {
    match self {
      GroupingStrategy::Individual => events
        .into_iter()
        .map(|e| (e.id.clone(), vec![e]))
        .collect(),
      GroupingStrategy::Chunks(size) => events
        .into_iter()
        .chunks(*size)
        .into_iter()
        .map(|c| (Ulid::new().to_string(), c.collect()))
        .collect(),
      GroupingStrategy::GroupByKey(f) => {
        let mut groups = HashMap::new();
        for event in events {
          let key = f(&event);
          groups.entry(key).or_insert_with(Vec::new).push(event);
        }
        groups.into_iter().collect()
      }
      GroupingStrategy::GroupByCorrelationId => {
        let mut groups = HashMap::new();
        for event in events {
          let key = event
            .payload
            .correlation_id
            .clone()
            .unwrap_or_else(|| event.id.clone());
          groups.entry(key).or_insert_with(Vec::new).push(event);
        }
        groups.into_iter().collect()
      }
      GroupingStrategy::All => vec![("*".to_string(), events)],
    }
  }
}

type EventHandlerFn<T> =
  ThreadSafeAsyncFn<(T, Arc<ApplicationContext>, Arc<EventSubscriberInteractor>)>;

#[derive(Clone)]
pub enum EventHandler {
  /**
   * The handler will be called in order for each event in the group.
   */
  Single(EventHandlerFn<EventData>),
  /**
   * The handler will be called once with all the events in the group.
   */
  Group(EventHandlerFn<Vec<EventData>>),
}

#[macro_export]
macro_rules! event_handler {
  ($f:expr) => {{
    fn f(
      (event, app_context, interactor): (
        EventData,
        Arc<ApplicationContext>,
        Arc<EventSubscriberInteractor>,
      ),
    ) -> impl futures::Future<Output = Result<(), anyhow::Error>> + Send + 'static {
      $f(event, app_context, interactor)
    }
    EventHandler::Single($crate::helpers::async_utils::async_callback(f))
  }};
}

#[macro_export]
macro_rules! group_event_handler {
  ($f:expr) => {{
    fn f(
      (event, app_context, interactor): (
        Vec<EventData>,
        Arc<ApplicationContext>,
        Arc<EventSubscriberInteractor>,
      ),
    ) -> impl futures::Future<Output = Result<(), anyhow::Error>> + Send + 'static {
      $f(event, app_context, interactor)
    }
    EventHandler::Group($crate::helpers::async_utils::async_callback(f))
  }};
}

impl EventHandler {
  pub async fn handle(
    &self,
    event_data: Vec<EventData>,
    app_context: Arc<ApplicationContext>,
    interactor: Arc<EventSubscriberInteractor>,
  ) -> Result<()> {
    match self {
      EventHandler::Single(f) => {
        for event in event_data {
          f((event, Arc::clone(&app_context), Arc::clone(&interactor))).await?;
        }
        Ok(())
      }
      EventHandler::Group(f) => f((event_data, app_context, interactor)).await,
    }
  }
}

#[derive(Builder)]
pub struct EventSubscriber {
  /**
   * A batch is the maximum number of events that will be pulled from the event store in one iteration.
   */
  #[builder(default = "1")]
  pub batch_size: usize,
  /*
   * A group is a set of events in a batch that will be passed to the handler together.
   * Items in the same group will be passed to the handler in the same call.
   * The handler will be called in parallel for each group in the batch.
   */
  #[builder(default)]
  pub grouping_strategy: GroupingStrategy,
  pub app_context: Arc<ApplicationContext>,
  #[builder(setter(into))]
  pub id: String,
  #[builder(setter(each(name = "topic")))]
  pub topics: Vec<Topic>,

  pub handler: EventHandler,
  #[builder(setter(skip), default = "self.get_default_interactor()?")]
  interactor: Arc<EventSubscriberInteractor>,
  #[builder(default = "Duration::from_secs(1)")]
  pub cooldown: Duration,
}

impl EventSubscriberBuilder {
  pub fn get_default_interactor(&self) -> Result<Arc<EventSubscriberInteractor>, String> {
    match (&self.app_context, &self.id) {
      (Some(app_context), Some(id)) => Ok(Arc::new(EventSubscriberInteractor::new(
        id.clone(),
        EventRepository::new(Arc::clone(&app_context.sqlite_connection)),
        Arc::clone(&app_context.scheduler),
      ))),
      _ => Err("SQLite connection and ID are required".to_string()),
    }
  }
}

impl EventSubscriber {
  pub async fn poll(&self) -> Result<Option<String>> {
    let event_list = self
      .interactor
      .get_events_after_cursor(&self.topics, self.batch_size)
      .await?;
    let topic_tags = self.topics.iter().map(|s| s.to_string()).join(",");
    debug!(
      topics = topic_tags.as_str(),
      subscriber_id = self.id,
      count = &event_list.rows.len(),
      "Subscriber polled"
    );
    let tail_cursor = event_list.tail_cursor();
    let groups = self.grouping_strategy.group(event_list.rows);

    join_all(groups.into_iter().map(|(group_id, group)| {
      let interactor = Arc::clone(&self.interactor);
      let app_context = Arc::clone(&self.app_context);
      let handler = self.handler.clone();
      let subscriber_id = self.id.clone();
      let stream_tags = topic_tags.clone();

      info!(
        topics = stream_tags.as_str(),
        subscriber_id,
        group_id = group_id,
        count = group.len(),
        "Processing group"
      );
      tokio::spawn(async move {
        let event_data = group
          .into_iter()
          .map(|row| EventData {
            entry_id: row.id,
            payload: row.payload,
            topic: row.topic,
          })
          .collect::<Vec<EventData>>();
        handler
          .handle(event_data, app_context, interactor)
          .await
          .inspect_err(|e| {
            error!(
              topics = stream_tags.as_str(),
              subscriber_id,
              error = e.to_string(),
              "Error processing group"
            );
          })?;
        Ok::<(), anyhow::Error>(())
      })
    }))
    .await;

    Ok(tail_cursor)
  }

  pub async fn sleep(&self) {
    sleep(self.cooldown).await;
  }

  pub async fn run(&self) -> Result<()> {
    loop {
      if self
        .interactor
        .get_status()
        .await?
        .unwrap_or(EventSubscriberStatus::Running)
        == EventSubscriberStatus::Running
      {
        if let Some(tail_cursor) = self.poll().await.inspect_err(|e| {
          error!(
            subscriber_id = self.id,
            error = e.to_string(),
            "Error polling stream"
          );
        })? {
          self.interactor.set_cursor(&tail_cursor).await?;
        }
      }
      self.sleep().await;
    }
  }
}
