use crate::files::file_metadata::file_name::FileName;
use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

impl TryFrom<f64> for Priority {
  type Error = anyhow::Error;

  fn try_from(value: f64) -> Result<Self, Self::Error> {
    Self::try_from(value as u32)
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

#[derive(Serialize, Deserialize, Debug, Clone, Default, Builder)]
#[builder(default, setter(strip_option, into))]
pub struct QueuePushParameters {
  pub file_name: FileName,
  pub priority: Option<Priority>,
  pub deduplication_key: Option<String>,
  pub correlation_id: Option<String>,
  pub metadata: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemKey {
  pub enqueue_time: NaiveDateTime,
  pub deduplication_key: String,
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

#[async_trait]
pub trait PriorityQueue {
  async fn get_size(&self) -> Result<u32>;

  async fn push(&self, params: QueuePushParameters) -> Result<()>;

  async fn empty(&self) -> Result<()>;

  async fn claim_item(&self) -> Result<Option<QueueItem>>;

  async fn delete_item(&self, key: ItemKey) -> Result<()>;

  async fn get_claimed_items(&self) -> Result<Vec<ClaimedQueueItem>>;

  async fn get_claimed_item_count(&self) -> Result<u32>;
}
