use crate::{files::file_metadata::file_name::FileName, helpers::priority::Priority};
use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
