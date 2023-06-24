use super::event::{EventPayload, Stream};
use anyhow::Result;
use r2d2::Pool;
use redis::{Client, Commands};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct EventPublisher {
  redis_connection_pool: Arc<Pool<Client>>,
}

impl EventPublisher {
  pub fn new(redis_connection_pool: Arc<Pool<Client>>) -> Self {
    Self {
      redis_connection_pool,
    }
  }

  pub fn publish(&self, stream: Stream, payload: EventPayload) -> Result<()> {
    let value: HashMap<String, String> = payload.into();
    self
      .redis_connection_pool
      .get()?
      .xadd_map(&stream.redis_key(), "*", value)?;
    Ok(())
  }
}
