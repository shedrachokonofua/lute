use crate::{
  files::file_metadata::{file_name::FileName, page_type::PageType},
  helpers::db::does_ft_index_exist,
};
use anyhow::Result;
use chrono::NaiveDateTime;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{
    FtAggregateOptions, FtCreateOptions, FtFieldSchema, FtFieldType, FtIndexDataType, FtReducer,
    FtSearchOptions, GenericCommands, HashCommands, SearchCommands,
  },
};
use std::{collections::HashMap, sync::Arc};
use tracing::info;

#[derive(Debug, Clone)]
pub struct FailedParseFile {
  pub file_name: FileName,
  pub error: String,
  pub last_attempted_at: NaiveDateTime,
}

impl From<HashMap<String, String>> for FailedParseFile {
  fn from(values: HashMap<String, String>) -> Self {
    let file_name = FileName(
      values
        .get("file_name")
        .expect("file_name not found")
        .to_string(),
    );
    let error = values.get("error").expect("error not found").to_string();
    let last_attempted_at = NaiveDateTime::parse_from_str(
      values
        .get("last_attempted_at")
        .expect("last_attempted_at not found"),
      "%Y-%m-%d %H:%M:%S%.f",
    )
    .expect("invalid last_attempted_at");
    Self {
      file_name,
      error,
      last_attempted_at,
    }
  }
}

impl From<FailedParseFile> for Vec<(String, String)> {
  fn from(val: FailedParseFile) -> Self {
    vec![
      ("file_name".to_string(), val.file_name.to_string()),
      (
        "page_type".to_string(),
        val.file_name.page_type().to_string(),
      ),
      ("error".to_string(), val.error),
      (
        "last_attempted_at".to_string(),
        val.last_attempted_at.to_string(),
      ),
    ]
  }
}

pub struct AggregatedError {
  pub error: String,
  pub count: u64,
}

impl From<Vec<(String, String)>> for AggregatedError {
  fn from(values: Vec<(String, String)>) -> Self {
    let error = values
      .iter()
      .find(|(k, _)| k == "error")
      .expect("error not found")
      .1
      .to_string();
    let count = values
      .iter()
      .find(|(k, _)| k == "count")
      .expect("count not found")
      .1
      .parse()
      .expect("invalid count");

    Self { error, count }
  }
}

pub struct FailedParseFilesRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl FailedParseFilesRepository {
  fn namespace(&self) -> &str {
    "failed_parse_files"
  }

  fn key(&self, file_name: &FileName) -> String {
    format!("{}:{}", self.namespace(), file_name.to_string())
  }

  fn search_index_name(&self) -> String {
    format!("{}_idx", self.namespace())
  }

  pub async fn put(&self, failed_parse_file: FailedParseFile) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    let items: Vec<(String, String)> = failed_parse_file.clone().try_into()?;
    connection
      .hset(self.key(&failed_parse_file.file_name), items)
      .await?;
    Ok(())
  }

  pub async fn remove(&self, file_name: &FileName) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection.del(self.key(file_name)).await?;
    Ok(())
  }

  pub async fn setup_index(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    if !does_ft_index_exist(&connection, &self.search_index_name()).await {
      info!("Creating new search index: {}", self.search_index_name());
      connection
        .ft_create(
          self.search_index_name(),
          FtCreateOptions::default()
            .on(FtIndexDataType::Hash)
            .prefix(format!("{}:", self.namespace())),
          [
            FtFieldSchema::identifier("error").field_type(FtFieldType::Tag),
            FtFieldSchema::identifier("page_type").field_type(FtFieldType::Tag),
          ],
        )
        .await?;
    }
    Ok(())
  }

  pub async fn find_many_by_error(&self, error: &str) -> Result<Vec<FailedParseFile>> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_search(
        self.search_index_name(),
        format!(
          "@error:{{ {} }}",
          error.replace(" ", "\\ ").replace(":", "\\:")
        ),
        FtSearchOptions::default().limit(0, 10000),
      )
      .await?;
    let file_names = result
      .results
      .iter()
      .map(|row| {
        let values = row
          .values
          .iter()
          .map(|(k, v)| (k.to_string(), v.to_string()))
          .collect::<HashMap<_, _>>();
        FailedParseFile::from(values)
      })
      .collect::<Vec<_>>();

    Ok(file_names)
  }

  pub async fn aggregate_errors(
    &self,
    _page_type: Option<PageType>,
  ) -> Result<Vec<AggregatedError>> {
    let connection = self.redis_connection_pool.get().await?;
    let result = connection
      .ft_aggregate(
        self.search_index_name(),
        "*",
        FtAggregateOptions::default().groupby("@error", FtReducer::count().as_name("count")),
      )
      .await?;
    let aggregates = result
      .results
      .iter()
      .map(|r| AggregatedError::from(r.to_owned()))
      .collect::<Vec<_>>();

    Ok(aggregates)
  }
}
