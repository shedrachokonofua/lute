use super::event_subscriber_repository::{
  EventSubscriberRepository, EventSubscriberRow, EventSubscriberStatus,
};
use crate::{proto, sqlite::SqliteConnection};
use futures::{try_join, Stream};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::time::sleep;
use tonic::{Request, Response, Status, Streaming};

impl Into<proto::EventSubscriberStatus> for EventSubscriberStatus {
  fn into(self) -> proto::EventSubscriberStatus {
    match self {
      EventSubscriberStatus::Paused => proto::EventSubscriberStatus::SubscriberPaused,
      EventSubscriberStatus::Running => proto::EventSubscriberStatus::SubscriberRunning,
      EventSubscriberStatus::Draining => proto::EventSubscriberStatus::SubscriberDraining,
    }
  }
}

impl Into<proto::EventSubscriberSnapshot> for EventSubscriberRow {
  fn into(self) -> proto::EventSubscriberSnapshot {
    proto::EventSubscriberSnapshot {
      id: self.id,
      cursor: self.cursor,
      status: Into::<proto::EventSubscriberStatus>::into(self.status).into(),
    }
  }
}

pub struct EventService {
  event_subscriber_repository: EventSubscriberRepository,
}

impl EventService {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self {
      event_subscriber_repository: EventSubscriberRepository::new(sqlite_connection),
    }
  }
}

#[tonic::async_trait]
impl proto::EventService for EventService {
  type StreamStream =
    Pin<Box<dyn Stream<Item = Result<proto::EventStreamReply, Status>> + Send + 'static>>;

  async fn set_cursor(
    &self,
    request: Request<proto::SetEventCursorRequest>,
  ) -> Result<Response<()>, Status> {
    let request = request.into_inner();
    self
      .event_subscriber_repository
      .set_cursor(&request.subscriber_id, &request.cursor)
      .await
      .map_err(|err| Status::internal(err.to_string()))?;
    Ok(Response::new(()))
  }

  async fn get_monitor(
    &self,
    _: Request<()>,
  ) -> Result<Response<proto::GetEventsMonitorReply>, Status> {
    let (event_count, subscribers, stream_tails) = try_join!(
      self.event_subscriber_repository.get_event_count(),
      self.event_subscriber_repository.get_subscribers(),
      self.event_subscriber_repository.get_stream_tails(),
    )
    .map_err(|err| Status::internal(err.to_string()))?;
    let monitor = proto::EventsMonitor {
      event_count: event_count as u32,
      subscribers: subscribers
        .into_iter()
        .map(|subscriber| subscriber.into())
        .collect(),
      streams: stream_tails
        .into_iter()
        .map(|(stream, tail)| proto::EventStreamSnapshot {
          id: stream.tag(),
          tail,
        })
        .collect(),
    };

    let reply = proto::GetEventsMonitorReply {
      monitor: Some(monitor.into()),
    };
    Ok(Response::new(reply))
  }

  async fn stream(
    &self,
    request: Request<Streaming<proto::EventStreamRequest>>,
  ) -> Result<Response<Self::StreamStream>, Status> {
    let mut input_stream: Streaming<proto::EventStreamRequest> = request.into_inner();
    let event_subscriber_repository = self.event_subscriber_repository.clone();
    let output_stream = async_stream::try_stream! {
      while let Ok(Some(event_stream_request)) = input_stream.message().await {
        loop {
          let stream_id = super::event::Stream::try_from(event_stream_request.stream_id.clone())
            .map_err(|err| Status::invalid_argument(err.to_string()))?;
          if let Some(cursor) = event_stream_request.cursor.clone() {
            event_subscriber_repository.set_cursor(
              &event_stream_request.subscriber_id,
              &cursor,
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
          }
          let event_list = event_subscriber_repository.get_events_after_cursor(
            &vec![stream_id.clone()],
            &event_stream_request.subscriber_id,
            event_stream_request.max_batch_size.unwrap_or(10) as usize,
          )
          .await
          .map_err(|err| Status::internal(err.to_string()))?;

          let tail_cursor = event_list.tail_cursor().clone();
          if let Some(tail_cursor) = tail_cursor {
            yield proto::EventStreamReply {
              items: event_list.rows.into_iter().map(|row| {
                proto::EventStreamItem {
                  entry_id: row.id.clone(),
                  payload: Some(row.payload.into()),
                  stream_id: stream_id.tag(),
                  timestamp: row.id.clone().split('-').next()
                    .expect("Invalid event stream item ID")
                    .parse::<u64>()
                    .expect("Invalid event stream item ID")
                }
              }).collect(),
              cursor: tail_cursor.clone(),
            };
            break;
          }
          sleep(Duration::from_secs(2)).await;
        }
      }
    };
    Ok(Response::new(Box::pin(output_stream) as Self::StreamStream))
  }
}
