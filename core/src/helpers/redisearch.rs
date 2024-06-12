use crate::proto;
use anyhow::Result;
use rustis::{
  bb8::{Pool, PooledConnection},
  client::PooledClientManager,
  commands::{FtCreateOptions, FtFieldSchema, SearchCommands},
};
use std::sync::Arc;
use tracing::warn;
use unidecode::unidecode;

#[derive(Debug)]
pub struct SearchPagination {
  pub offset: Option<usize>,
  pub limit: Option<usize>,
}

impl From<proto::SearchPagination> for SearchPagination {
  fn from(value: proto::SearchPagination) -> Self {
    SearchPagination {
      offset: value.offset.map(|i| i as usize),
      limit: value.limit.map(|i| i as usize),
    }
  }
}

pub async fn does_ft_index_exist<'a>(
  connection: &PooledConnection<'a, PooledClientManager>,
  index_name: &String,
) -> bool {
  match connection.ft_info(index_name).await {
    Ok(_) => true,
    Err(err) => {
      warn!("Failed to check if index exists: {}", err);
      !err.to_string().contains("Unknown Index name")
    }
  }
}

pub fn escape_search_query_text(input: &str) -> String {
  unidecode(input.trim())
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() {
        c.to_string()
      } else {
        " ".to_string()
      }
    })
    .collect()
}

pub fn escape_tag_value(input: &str) -> String {
  input
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || c == '…' {
        c.to_string()
      } else if c.is_ascii() {
        format!("\\{}", c)
      } else {
        // Convert non-ASCII chars to UTF-8
        c.to_string()
          .as_bytes()
          .iter()
          .map(|b| format!("{:02x}", b))
          .collect::<Vec<String>>()
          .join("")
      }
    })
    .collect()
}

pub struct SearchIndexVersionManager {
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  pub version: u32,
  pub base_name: String,
}

impl SearchIndexVersionManager {
  pub fn new(
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    version: u32,
    base_name: String,
  ) -> Self {
    Self {
      redis_connection_pool,
      version,
      base_name,
    }
  }

  pub fn latest_index_name(&self) -> String {
    format!("{}-{}", self.base_name, self.version)
  }

  pub async fn delete_old_indexes(&self) -> Result<()> {
    let pool = Arc::clone(&self.redis_connection_pool);
    let connection = pool.get().await?;
    if self.version <= 1 {
      for version in 1..self.version {
        let index_name = format!("{}-{}", self.base_name, version);
        if does_ft_index_exist(&connection, &index_name).await {
          connection.ft_dropindex(index_name, false).await?;
        }
      }
    }
    Ok(())
  }

  pub async fn setup_index(
    &self,
    create_options: FtCreateOptions,
    latest_schema: Vec<FtFieldSchema>,
  ) -> Result<()> {
    let pool = Arc::clone(&self.redis_connection_pool);
    let connection = pool.get().await?;
    if !does_ft_index_exist(&connection, &self.latest_index_name()).await {
      connection
        .ft_create(&self.latest_index_name(), create_options, latest_schema)
        .await?;
      self.delete_old_indexes().await?
    };
    Ok(())
  }
}

pub fn get_tag_query<T: ToString>(tag: &str, items: &[T]) -> String {
  if !items.is_empty() {
    format!(
      "{}:{{{}}} ",
      tag,
      items
        .iter()
        .map(|item| escape_tag_value(item.to_string().as_str()))
        .collect::<Vec<String>>()
        .join("|")
    )
  } else {
    String::from("")
  }
}

pub fn get_min_num_query(tag: &str, min: Option<usize>) -> String {
  if let Some(min) = min {
    format!("{}:[{}, +inf] ", tag, min)
  } else {
    String::from("")
  }
}

pub fn get_max_num_query(tag: &str, min: Option<usize>) -> String {
  if let Some(min) = min {
    format!("{}:[-inf, {}] ", tag, min)
  } else {
    String::from("")
  }
}

pub fn get_num_range_query(tag: &str, min: Option<u32>, max: Option<u32>) -> String {
  match (min, max) {
    (Some(min), Some(max)) => format!("{}:[{}, {}] ", tag, min, max),
    (Some(min), None) => format!("{}:[{}, +inf] ", tag, min),
    (None, Some(max)) => format!("{}:[-inf, {}] ", tag, max),
    (None, None) => String::from(""),
  }
}
