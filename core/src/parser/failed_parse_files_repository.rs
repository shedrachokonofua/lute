use crate::files::file_metadata::{file_name::FileName, page_type::PageType};
use anyhow::Result;
use chrono::NaiveDateTime;
use r2d2::Pool;
use redis::{cmd, Client, Commands, Value};
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
    let last_attempted_at: NaiveDateTime = values
      .get("last_attempted_at")
      .expect("last_attempted_at not found")
      .parse()
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

pub struct FailedParseFilesRepository {
  pub redis_connection_pool: Arc<Pool<Client>>,
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

  pub fn put(&self, failed_parse_file: FailedParseFile) -> Result<()> {
    let mut connection = self.redis_connection_pool.get()?;
    let items: Vec<(String, String)> = failed_parse_file.clone().try_into()?;
    connection.hset_multiple(self.key(&failed_parse_file.file_name), &items)?;
    Ok(())
  }

  pub fn remove(&self, file_name: &FileName) -> Result<()> {
    let mut connection = self.redis_connection_pool.get()?;
    connection.del(self.key(file_name))?;
    Ok(())
  }

  pub fn setup_index(&self) -> Result<()> {
    let mut connection = self.redis_connection_pool.get()?;
    let index_info: Result<Value, _> = cmd("FT.INFO")
      .arg(self.search_index_name())
      .query(&mut connection);

    if let Err(err) = index_info {
      if err.to_string().contains("Unknown: Index name") {
        info!("Creating new search index: {}", self.search_index_name());

        cmd("FT.CREATE")
          .arg(self.search_index_name())
          .arg("ON")
          .arg("HASH")
          .arg("PREFIX")
          .arg("1")
          .arg(format!("{}:", self.namespace()))
          .arg("SCHEMA")
          .arg("error")
          .arg("TEXT")
          .arg("page_type")
          .arg("TEXT")
          .query(&mut connection)?;
      } else {
        return Err(err.into());
      }
    }

    Ok(())
  }

  pub fn aggregate_errors(&self, page_type: Option<PageType>) -> Result<Vec<AggregatedError>> {
    let mut connection = self.redis_connection_pool.get()?;

    let results: Vec<HashMap<String, String>> = cmd("FT.AGGREGATE")
      .arg(self.search_index_name())
      .arg("*")
      .arg("GROUPBY")
      .arg("1")
      .arg("@error")
      .arg("REDUCE")
      .arg("COUNT")
      .arg("0")
      .arg("AS")
      .arg("count")
      .query(&mut connection)?;

    Ok(
      results
        .into_iter()
        .map(|result| AggregatedError {
          error: result.get("error").unwrap().to_string(),
          count: result.get("count").unwrap().parse().unwrap(),
        })
        .collect(),
    )
  }
}
