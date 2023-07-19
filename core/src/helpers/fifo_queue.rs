use anyhow::Result;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{BlockingCommands, ListCommands},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};

pub struct FifoQueue<T: Serialize + DeserializeOwned + Send + Debug + 'static> {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  key: &'static str,
  _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned + Send + Debug + 'static> FifoQueue<T> {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>, key: &'static str) -> Self {
    Self {
      redis_connection_pool,
      key,
      _phantom: PhantomData,
    }
  }

  pub async fn push(&self, value: &T) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection
      .rpush(
        self.key,
        [serde_json::to_string(value).map_err(|error| anyhow::anyhow!(error))?],
      )
      .await?;
    Ok(())
  }

  pub async fn push_many(&self, values: Vec<T>) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection
      .rpush(
        self.key,
        values
          .iter()
          .map(|value| {
            serde_json::to_string(value)
              .map_err(|error| anyhow::anyhow!(error))
              .unwrap()
          })
          .collect::<Vec<String>>(),
      )
      .await?;
    Ok(())
  }

  pub async fn recv(&self) -> Result<T> {
    let connection = self.redis_connection_pool.get().await?;
    let result: (String, String) = connection.blpop(self.key, 0.0).await?.unwrap();
    let value: T = serde_json::from_str(&result.1).map_err(|error| {
      tracing::warn!(
        error = error.to_string().as_str(),
        "Failed to deserialize value"
      );
      anyhow::anyhow!(error)
    })?;
    Ok(value)
  }
}
