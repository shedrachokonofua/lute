use super::{
  event_subscriber_repository::EventSubscriberRepository,
  sqlite_event_subscriber_repository::SqliteEventSubscriberRepository,
};
use crate::proto;
use futures::Stream;
use std::{pin::Pin, sync::Arc};
use tonic::{Request, Response, Status, Streaming};

pub struct EventService {
  sqlite_connection: Arc<tokio_rusqlite::Connection>,
}

impl EventService {
  pub fn new(sqlite_connection: Arc<tokio_rusqlite::Connection>) -> Self {
    Self { sqlite_connection }
  }
}

#[tonic::async_trait]
impl proto::EventService for EventService {
  type StreamStream =
    Pin<Box<dyn Stream<Item = Result<proto::EventStreamReply, Status>> + Send + 'static>>;

  async fn stream(
    &self,
    request: Request<Streaming<proto::EventStreamRequest>>,
  ) -> Result<Response<Self::StreamStream>, Status> {
    let mut input_stream: Streaming<proto::EventStreamRequest> = request.into_inner();
    let event_subscriber_repository =
      SqliteEventSubscriberRepository::new(Arc::clone(&self.sqlite_connection));
    let output_stream = async_stream::try_stream! {
      while let Ok(Some(event_stream_request)) = input_stream.message().await {
        loop {
          let stream_id = super::event::Stream::try_from(event_stream_request.stream_id.clone())
            .map_err(|err| Status::invalid_argument(err.to_string()))?;
          if let Some(cursor) = event_stream_request.cursor.clone() {
            event_subscriber_repository.set_cursor(
              &stream_id,
              &event_stream_request.subscriber_id,
              &cursor,
            )
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
          }
          let event_list = event_subscriber_repository.get_events_after_cursor(
            &stream_id,
            &event_stream_request.subscriber_id,
            event_stream_request.max_batch_size.unwrap_or(10) as usize,
            Some(10000)
          )
          .await
          .map_err(|err| Status::internal(err.to_string()))?;

          let tail_cursor = event_list.tail_cursor().clone();
          if let Some(tail_cursor) = tail_cursor {
            yield proto::EventStreamReply {
              items: event_list.events.into_iter().map(|(id, payload)| {
                proto::EventStreamItem {
                  entry_id: id.clone(),
                  payload: Some(payload.into()),
                  stream_id: stream_id.tag(),
                  timestamp: id.clone().split('-').next()
                    .expect("Invalid event stream item ID")
                    .parse::<u64>()
                    .expect("Invalid event stream item ID")
                }
              }).collect(),
              cursor: tail_cursor.clone(),
            };
            break;
          }
        }
      }
    };
    Ok(Response::new(Box::pin(output_stream) as Self::StreamStream))
  }
}
