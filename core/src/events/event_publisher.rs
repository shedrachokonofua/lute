use anyhow::Result;
use r2d2::PooledConnection;
use redis::{Client, Commands};

use super::{
  event::{EventPayload, EventTag},
  event_stream::get_stream_key,
};

pub struct EventPublisher {
  redis_connection: PooledConnection<Client>,
}

impl EventPublisher {
  pub fn new(redis_connection: PooledConnection<Client>) -> Self {
    Self { redis_connection }
  }

  pub fn publish(&mut self, payload: EventPayload) -> Result<()> {
    let key = get_stream_key(&payload.event);
    let value: Vec<(String, String)> = payload.into();
    println!("Publishing to stream: {}", key);
    self.redis_connection.xadd(key, "*", &value)?;
    Ok(())
  }
}
