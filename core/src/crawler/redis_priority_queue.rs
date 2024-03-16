use super::priority_queue::{
  ClaimedQueueItem, ItemKey, Priority, PriorityQueue, QueueItem, QueuePushParameters,
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::DateTime;
use futures::future::join_all;
use rustis::{
  bb8::Pool,
  client::{BatchPreparedCommand, PooledClientManager},
  commands::{
    GenericCommands, HashCommands, SortedSetCommands, StringCommands, ZAddOptions, ZRangeOptions,
  },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::Mutex;
use tracing::{info, instrument, warn};

#[derive(Serialize, Deserialize, Clone)]
pub struct QueueItemSetRecord {
  pub file_name: FileName,
  pub correlation_id: Option<String>,
  pub metadata: Option<HashMap<String, String>>,
}

impl ToString for ItemKey {
  fn to_string(&self) -> String {
    format!(
      "{}:DELIMETER:{}",
      self.enqueue_time.and_utc().timestamp(),
      self.deduplication_key
    )
  }
}

impl FromStr for ItemKey {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let parts: Vec<&str> = s.split(":DELIMETER:").collect();
    if parts.len() != 2 {
      bail!("Invalid queue item member string");
    }
    let enqueue_time = DateTime::from_timestamp(parts[0].parse::<i64>()?, 0);
    if enqueue_time.is_none() {
      bail!("Invalid queue item member string");
    }
    let deduplication_key = parts[1].to_string();
    Ok(ItemKey {
      enqueue_time: enqueue_time.unwrap().naive_utc(),
      deduplication_key,
    })
  }
}

#[derive(Debug)]
pub struct RedisPriorityQueue {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub max_size: u32,
  pub claim_ttl_seconds: u32,
  push_lock: Mutex<()>,
  claim_lock: Mutex<()>,
}

impl RedisPriorityQueue {
  pub fn new(
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    max_size: u32,
    claim_ttl_seconds: u32,
  ) -> Self {
    Self {
      redis_connection_pool,
      max_size,
      claim_ttl_seconds,
      push_lock: Mutex::new(()),
      claim_lock: Mutex::new(()),
    }
  }

  fn redis_key(&self) -> &str {
    "crawler:queue"
  }

  fn item_set_key(&self) -> String {
    format!("{}:items", self.redis_key())
  }

  fn claimed_item_key_str(&self, key: &str) -> String {
    format!("{}:claimed:{}", self.redis_key(), key)
  }

  fn claimed_item_key(&self, key: &ItemKey) -> String {
    self.claimed_item_key_str(key.to_string().as_str())
  }

  #[instrument(skip(self))]
  async fn contains(&self, key: &str) -> Result<bool> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection.hexists(self.item_set_key(), key).await?;
    Ok(result)
  }

  async fn is_full(&self) -> Result<bool> {
    let result = self.get_size().await? >= self.max_size;
    Ok(result)
  }

  #[instrument(skip(self))]
  async fn get_item(&self, key: &ItemKey) -> Result<Option<QueueItem>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection
      .hget(self.item_set_key(), &key.deduplication_key)
      .await?;
    if result.is_none() {
      return Ok(None);
    }
    let item_set_record: QueueItemSetRecord = serde_json::from_str(&result.unwrap())?;
    let priority_score: Option<f64> = connection
      .zscore(self.redis_key(), &key.to_string())
      .await?;
    let priority_score = priority_score.unwrap_or(Priority::Standard as u32 as f64);

    Ok(Some(QueueItem {
      item_key: key.clone(),
      enqueue_time: key.enqueue_time,
      deduplication_key: key.deduplication_key.clone(),
      file_name: item_set_record.file_name,
      correlation_id: item_set_record.correlation_id,
      metadata: item_set_record.metadata,
      priority: Priority::try_from(priority_score)?,
    }))
  }

  #[instrument(skip(self))]
  async fn is_claimed(&self, key: &ItemKey) -> Result<bool> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection.exists(self.claimed_item_key(key)).await? == 1;
    Ok(result)
  }

  #[instrument(skip(self))]
  async fn at(&self, position: isize) -> Result<Option<QueueItem>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Vec<String> = connection
      .zrange(
        self.redis_key(),
        position,
        position,
        ZRangeOptions::default(),
      )
      .await?;
    let item_key = result.first();
    if item_key.is_none() {
      return Ok(None);
    }
    self.get_item(&item_key.unwrap().parse::<ItemKey>()?).await
  }

  #[instrument(skip(self))]
  async fn get_next_unclaimed_item(&self) -> Result<Option<QueueItem>> {
    let mut index = 0;
    loop {
      let item = self.at(index).await?;
      if item.is_none() {
        return Ok(None);
      }
      let item = item.unwrap();
      if !self.is_claimed(&item.item_key).await? {
        return Ok(Some(item));
      }
      index += 1;
    }
  }
}

#[async_trait]
impl PriorityQueue for RedisPriorityQueue {
  #[instrument(skip(self))]
  async fn get_size(&self) -> Result<u32> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection.zcard(self.redis_key()).await?;
    Ok(result as u32)
  }

  #[instrument(skip(self))]
  async fn push(&self, params: QueuePushParameters) -> Result<()> {
    let _ = self.push_lock.lock().await;
    let deduplication_key = params
      .deduplication_key
      .unwrap_or_else(|| params.file_name.to_string());

    if self.contains(&deduplication_key).await? {
      warn!("Item already exists in queue, skipping");
      return Ok(());
    }

    if self.is_full().await? {
      bail!("Queue is full");
    }

    let connection = self.redis_connection_pool.get().await?;
    let mut transaction = connection.create_transaction();
    transaction
      .zadd(
        self.redis_key(),
        (
          params.priority.unwrap_or(Priority::Standard) as u32 as f64,
          ItemKey {
            enqueue_time: chrono::Utc::now().naive_utc(),
            deduplication_key: deduplication_key.clone(),
          }
          .to_string(),
        ),
        ZAddOptions::default(),
      )
      .forget();
    transaction
      .hset(
        self.item_set_key(),
        (
          &deduplication_key,
          serde_json::to_string(&QueueItemSetRecord {
            file_name: params.file_name,
            metadata: params.metadata,
            correlation_id: params.correlation_id,
          })?,
        ),
      )
      .queue();
    transaction.execute().await?;

    Ok(())
  }

  #[instrument(skip(self))]
  async fn empty(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection
      .del([self.redis_key(), self.item_set_key().as_str()])
      .await?;
    let claims: Vec<String> = connection.keys(self.claimed_item_key_str("*")).await?;
    let mut transaction = connection.create_transaction();
    for claim in claims {
      transaction.del(claim).forget();
    }
    transaction.execute().await?;

    Ok(())
  }

  #[instrument(skip(self))]
  async fn claim_item(&self) -> Result<Option<QueueItem>> {
    let _ = self.claim_lock.lock().await;
    let item = self.get_next_unclaimed_item().await?;
    if item.is_none() {
      return Ok(None);
    }
    info!("Found item to claim {:?}", item);
    let item = item.unwrap();

    let connection = self.redis_connection_pool.get().await?;
    connection
      .setex(
        self.claimed_item_key(&item.item_key),
        self.claim_ttl_seconds as u64,
        "1",
      )
      .await?;
    Ok(Some(item))
  }

  #[instrument(skip(self))]
  async fn delete_item(&self, key: ItemKey) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    let mut transaction = connection.create_transaction();
    transaction
      .zrem(self.redis_key(), &key.to_string())
      .forget();
    transaction
      .hdel(self.item_set_key(), &key.deduplication_key)
      .forget();
    transaction.del(self.claimed_item_key(&key)).queue();
    transaction.execute().await?;
    Ok(())
  }

  #[instrument(skip(self))]
  async fn get_claimed_items(&self) -> Result<Vec<ClaimedQueueItem>> {
    let connection = self.redis_connection_pool.get().await?;
    let claimed_redis_keys: Vec<String> = connection.keys(self.claimed_item_key_str("*")).await?;
    let claimed_keys = claimed_redis_keys
      .iter()
      .map(|key| {
        ItemKey::from_str(key.replace(&self.claimed_item_key_str(""), "").as_str()).unwrap()
      })
      .collect::<Vec<ItemKey>>();

    let item_futures: Vec<_> = claimed_keys.iter().map(|key| self.get_item(key)).collect();
    let items: Vec<QueueItem> = join_all(item_futures)
      .await
      .iter()
      .filter_map(|item_result| match item_result {
        Ok(Some(item)) => Some(item.clone()),
        _ => None,
      })
      .collect::<Vec<QueueItem>>();

    let claimed_items = join_all(items.iter().map(|item| async {
      ClaimedQueueItem {
        item: item.clone(),
        claim_ttl_seconds: connection
          .ttl(self.claimed_item_key(&item.item_key))
          .await
          .unwrap() as u32,
      }
    }))
    .await;

    Ok(claimed_items)
  }

  #[instrument(skip(self))]
  async fn get_claimed_item_count(&self) -> Result<u32> {
    let connection = self.redis_connection_pool.get().await?;
    let claimed_redis_keys: Vec<String> = connection.keys(self.claimed_item_key_str("*")).await?;
    Ok(claimed_redis_keys.len() as u32)
  }
}
