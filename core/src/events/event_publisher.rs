use super::event::{EventPayload, Stream};
use anyhow::Result;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{StreamCommands, XAddOptions},
};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct EventPublisher {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl EventPublisher {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      redis_connection_pool,
    }
  }

  pub async fn publish(&self, stream: Stream, payload: EventPayload) -> Result<()> {
    let value: HashMap<String, String> = payload.into();
    let _: String = self
      .redis_connection_pool
      .get()
      .await?
      .xadd(&stream.redis_key(), "*", value, XAddOptions::default())
      .await?;
    Ok(())
  }
}
