use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{hash::Hash, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
  spawn,
  sync::mpsc::{unbounded_channel, UnboundedSender},
  time::{timeout, Instant},
};
use tracing::error;

#[derive(Error, Debug, Clone)]
#[error("Loader error: {msg}")]
pub struct LoaderError {
  pub msg: String,
}

#[async_trait]
pub trait Loader {
  type Key: Clone + Eq + Hash + Send + Sync;
  type Value: Clone + Send + Sync;
  async fn load(&self, keys: &[Self::Key]) -> Vec<Result<Self::Value, LoaderError>>;
}

#[derive(Clone)]
pub struct BatchLoaderConfig {
  pub batch_size: usize,
  pub time_limit: Duration,
}

pub struct BatchLoader<T: Loader> {
  sender: UnboundedSender<(T::Key, UnboundedSender<Result<T::Value, LoaderError>>)>,
}

impl<T: Loader + Send + Sync + 'static> BatchLoader<T> {
  pub fn new(loader: T, config: BatchLoaderConfig) -> Self {
    let (sender, mut receiver) =
      unbounded_channel::<(T::Key, UnboundedSender<Result<T::Value, LoaderError>>)>();
    let loader = Arc::new(loader);
    spawn(async move {
      let mut next_batch = Vec::new();
      let mut last_execution = Instant::now();

      loop {
        let mut last_received = Instant::now();
        while let Ok(Some((key, sender))) =
          timeout(config.time_limit - last_received.elapsed(), receiver.recv()).await
        {
          last_received = Instant::now();
          next_batch.push((key, sender));

          if next_batch.len() >= config.batch_size || last_execution.elapsed() >= config.time_limit
          {
            break;
          }
        }

        if !next_batch.is_empty() {
          let batch = next_batch.drain(..).collect::<Vec<_>>();
          let loader = loader.clone();
          last_execution = Instant::now();

          spawn(async move {
            let keys: Vec<T::Key> = batch
              .iter()
              .map(|(key, _): &(T::Key, _)| key.clone())
              .collect();
            let values = loader.load(&keys).await;

            for ((_, sender), value) in batch.into_iter().zip(values.into_iter()) {
              if let Err(e) = sender.send(value) {
                error!("Failed to send value: {}", e);
              }
            }
          });
        }
      }
    });

    BatchLoader { sender }
  }

  pub async fn load(&self, key: T::Key) -> Result<T::Value> {
    let (sender, mut receiver) = unbounded_channel::<Result<T::Value, LoaderError>>();
    self.sender.send((key, sender))?;
    receiver
      .recv()
      .await
      .unwrap_or(Err(LoaderError {
        msg: "Failed to receive value".to_string(),
      }))
      .map_err(|e| anyhow!("Failed to receive value: {}", e))
  }
}
