use anyhow::Result;
use async_trait::async_trait;
use std::{hash::Hash, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
  spawn,
  sync::mpsc::{unbounded_channel, UnboundedSender},
  time::{sleep, Instant},
};
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

    let loader_clone = loader.clone();
    let config_clone = config.clone();
    spawn(async move {
      let mut batch = Vec::new();
      let mut last_execution = Instant::now();

      loop {
        // Collect incoming requests
        while let Ok((key, sender)) = receiver.try_recv() {
          batch.push((key, sender));

          // Check if the batch size or time limit is reached
          if batch.len() >= config_clone.batch_size
            || last_execution.elapsed() >= config_clone.time_limit
          {
            break;
          }
        }

        // Execute the batch if there are any requests
        if !batch.is_empty() {
          let keys: Vec<T::Key> = batch
            .iter()
            .map(|(key, _): &(T::Key, _)| key.clone())
            .collect();
          let values = loader_clone.load(&keys).await;

          // Send the loaded values back to the corresponding senders
          for ((_, sender), value) in batch.clone().into_iter().zip(values.into_iter()) {
            sender.send(value).unwrap();
          }

          last_execution = Instant::now();
          batch.clear();
        }

        // Sleep for a short duration to avoid busy waiting
        sleep(Duration::from_millis(10)).await;
      }
    });

    BatchLoader { sender }
  }

  pub async fn load(&self, key: T::Key) -> Result<T::Value> {
    let (sender, mut receiver) = unbounded_channel::<Result<T::Value, LoaderError>>();
    self.sender.send((key, sender)).unwrap();
    receiver.recv().await.unwrap().map_err(|e| e.into())
  }
}
