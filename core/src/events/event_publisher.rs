use super::event::{EventPayload, Stream};
use crate::settings::Settings;
use anyhow::Result;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{StreamCommands, XAddOptions},
};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

#[derive(Debug, Clone)]
pub struct EventPublisher {
  pub settings: Arc<Settings>,
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl EventPublisher {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
  ) -> Self {
    Self {
      settings,
      redis_connection_pool,
    }
  }

  async fn inner_publish(&self, stream: Stream, payload: EventPayload) -> Result<()> {
    let value: HashMap<String, String> = payload.into();
    let _: String = self
      .redis_connection_pool
      .get()
      .await?
      .xadd(&stream.redis_key(), "*", value, XAddOptions::default())
      .await?;
    Ok(())
  }

  pub async fn publish(&self, stream: Stream, payload: EventPayload) -> Result<()> {
    self.inner_publish(stream, payload.clone()).await?;
    if self.settings.enable_replication_stream {
      self
        .inner_publish(Stream::Replication, payload)
        .await
        .map_err(|err| {
          error!("Failed to publish to replication stream: {}", err);
          err
        })?;
    }
    Ok(())
  }
}
