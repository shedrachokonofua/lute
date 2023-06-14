use crate::files::file_metadata::file_name::FileName;
use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use r2d2::Pool;
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Priority {
  Express = 0,
  High = 1,
  Standard = 2,
  Low = 3,
}

impl TryFrom<u32> for Priority {
  type Error = anyhow::Error;

  fn try_from(value: u32) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(Priority::Express),
      1 => Ok(Priority::High),
      2 => Ok(Priority::Standard),
      3 => Ok(Priority::Low),
      _ => bail!("Invalid priority value"),
    }
  }
}

impl ToString for Priority {
  fn to_string(&self) -> String {
    match self {
      Priority::Express => "express".to_string(),
      Priority::High => "high".to_string(),
      Priority::Standard => "standard".to_string(),
      Priority::Low => "low".to_string(),
    }
  }
}

pub struct QueuePushParameters {
  pub file_name: FileName,
  pub priority: Option<Priority>,
  pub deduplication_key: Option<String>,
  pub correlation_id: Option<String>,
  pub metadata: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QueueItemSetRecord {
  pub file_name: FileName,
  pub correlation_id: Option<String>,
  pub metadata: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemKey {
  pub enqueue_time: NaiveDateTime,
  pub deduplication_key: String,
}

impl ToString for ItemKey {
  fn to_string(&self) -> String {
    format!(
      "{}:DELIMETER:{}",
      self.enqueue_time.timestamp(),
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
    let enqueue_time = NaiveDateTime::from_timestamp_opt(parts[0].parse::<i64>()?, 0);
    if enqueue_time.is_none() {
      bail!("Invalid queue item member string");
    }
    let deduplication_key = parts[1].to_string();
    Ok(ItemKey {
      enqueue_time: enqueue_time.unwrap(),
      deduplication_key,
    })
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueueItem {
  pub item_key: ItemKey,
  pub enqueue_time: NaiveDateTime,
  pub deduplication_key: String,
  pub file_name: FileName,
  pub priority: Priority,
  pub correlation_id: Option<String>,
  pub metadata: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct ClaimedQueueItem {
  pub item: QueueItem,
  pub claim_ttl_seconds: u32,
}

pub struct PriorityQueue {
  pub redis_connection_pool: Arc<Pool<Client>>,
  pub max_size: u32,
  pub claim_ttl_seconds: u32,
  push_lock: Mutex<()>,
  claim_lock: Mutex<()>,
}

impl PriorityQueue {
  pub fn new(
    redis_connection_pool: Arc<Pool<Client>>,
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

  pub fn redis_key(&self) -> String {
    "crawler:queue".to_string()
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

  fn contains(&self, key: &str) -> Result<bool> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: bool = connection.hexists(self.item_set_key(), key)?;
    Ok(result)
  }

  pub fn get_size(&self) -> Result<u32> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: u32 = connection.zcard(&self.redis_key())?;
    Ok(result)
  }

  fn is_full(&self) -> Result<bool> {
    let result = self.get_size()? >= self.max_size;
    Ok(result)
  }

  pub async fn push(&self, params: QueuePushParameters) -> Result<()> {
    let _ = self.push_lock.lock().await;
    let deduplication_key = params
      .deduplication_key
      .unwrap_or_else(|| params.file_name.to_string());

    if self.contains(&deduplication_key)? {
      bail!("Item already exists in queue");
    }

    if self.is_full()? {
      bail!("Queue is full");
    }

    let mut connection = self.redis_connection_pool.get()?;
    let mut transaction = redis::pipe();
    transaction.zadd(
      &self.redis_key(),
      ItemKey {
        enqueue_time: chrono::Utc::now().naive_utc(),
        deduplication_key: deduplication_key.clone(),
      }
      .to_string(),
      params.priority.unwrap_or(Priority::Standard) as u32,
    );
    transaction.hset(
      self.item_set_key(),
      &deduplication_key,
      serde_json::to_string(&QueueItemSetRecord {
        file_name: params.file_name,
        metadata: params.metadata,
        correlation_id: params.correlation_id,
      })?,
    );
    transaction.query(&mut connection)?;

    Ok(())
  }

  pub fn get_item(&self, key: &ItemKey) -> Result<Option<QueueItem>> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: Option<String> = connection.hget(self.item_set_key(), &key.deduplication_key)?;
    if result.is_none() {
      return Ok(None);
    }
    let item_set_record: QueueItemSetRecord = serde_json::from_str(&result.unwrap())?;
    let priority_score: Option<u32> = connection.zscore(&self.redis_key(), &key.to_string())?;
    let priority_score = priority_score.unwrap_or(Priority::Standard as u32);

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

  pub fn is_claimed(&self, key: &ItemKey) -> Result<bool> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: bool = connection.exists(self.claimed_item_key(key))?;
    Ok(result)
  }

  pub fn at(&self, position: isize) -> Result<Option<QueueItem>> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: Vec<String> = connection.zrange(&self.redis_key(), position, position)?;
    let item_key = result.first();
    if item_key.is_none() {
      return Ok(None);
    }
    self.get_item(&item_key.unwrap().parse::<ItemKey>()?)
  }

  pub fn peek(&self) -> Result<Option<QueueItem>> {
    self.at(0)
  }

  pub fn empty(&self) -> Result<()> {
    let mut connection = self.redis_connection_pool.get()?;
    let mut transaction = redis::pipe();
    transaction.del(&self.redis_key());
    transaction.del(self.item_set_key());
    transaction.query(&mut connection)?;
    Ok(())
  }

  pub fn get_next_unclaimed_item(&self) -> Result<Option<QueueItem>> {
    let mut index = 0;
    loop {
      let item = self.at(index)?;
      if item.is_none() {
        return Ok(None);
      }
      let item = item.unwrap();
      if !self.is_claimed(&item.item_key)? {
        return Ok(Some(item));
      }
      index += 1;
    }
  }

  pub async fn claim_item(&self) -> Result<Option<QueueItem>> {
    let _ = self.claim_lock.lock().await;
    let item = self.get_next_unclaimed_item()?;
    if item.is_none() {
      return Ok(None);
    }
    let item = item.unwrap();

    let mut connection = self.redis_connection_pool.get()?;
    connection.set_ex(
      self.claimed_item_key(&item.item_key),
      "1",
      self.claim_ttl_seconds as usize,
    )?;

    Ok(Some(item))
  }

  pub fn delete_item(&self, key: ItemKey) -> Result<()> {
    let mut connection = self.redis_connection_pool.get()?;
    let mut transaction = redis::pipe();
    transaction.zrem(&self.redis_key(), &key.to_string());
    transaction.hdel(self.item_set_key(), &key.deduplication_key);
    transaction.del(self.claimed_item_key(&key));
    transaction.query(&mut connection)?;
    Ok(())
  }

  pub fn get_claimed_items(&self) -> Result<Vec<ClaimedQueueItem>> {
    let mut connection = self.redis_connection_pool.get()?;
    let claimed_redis_keys: Vec<String> = connection.keys(self.claimed_item_key_str("*"))?;
    let claimed_keys = claimed_redis_keys
      .iter()
      .map(|key| {
        ItemKey::from_str(key.replace(&self.claimed_item_key_str(""), "").as_str()).unwrap()
      })
      .collect::<Vec<ItemKey>>();

    let items_opt = claimed_keys
      .iter()
      .map(|key| self.get_item(key))
      .collect::<Result<Vec<Option<QueueItem>>>>()?;

    let items = items_opt
      .iter()
      .filter_map(|item| item.clone())
      .collect::<Vec<QueueItem>>();

    let claimed_items = items
      .iter()
      .map(|item| ClaimedQueueItem {
        item: item.clone(),
        claim_ttl_seconds: connection
          .ttl(self.claimed_item_key(&item.item_key))
          .unwrap(),
      })
      .collect::<Vec<ClaimedQueueItem>>();

    Ok(claimed_items)
  }

  pub fn get_claimed_item_count(&self) -> Result<u32> {
    let mut connection = self.redis_connection_pool.get()?;
    let claimed_redis_keys: Vec<String> = connection.keys(self.claimed_item_key_str("*"))?;
    Ok(claimed_redis_keys.len() as u32)
  }
}
