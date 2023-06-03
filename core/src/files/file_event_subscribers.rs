use std::sync::Arc;

use crate::events::{
  event::{Event, EventPayload, Stream},
  event_subscriber::EventSubscriber,
};
use anyhow::Result;
use r2d2::Pool;
use redis::Client;

fn print_file_saved(payload: EventPayload) -> Result<()> {
  match payload.event {
    Event::FileSaved { file_id, file_name } => {
      println!("File saved: {} : {}", file_name, file_id);
    }
  }
  Ok(())
}

pub fn get_file_event_subscribers(redis_pool: Arc<Pool<Client>>) -> Vec<EventSubscriber> {
  vec![EventSubscriber {
    redis_pool: redis_pool.clone(),
    id: "file_saved".to_string(),
    stream: Stream::File,
    handle: Box::new(print_file_saved),
  }]
}
