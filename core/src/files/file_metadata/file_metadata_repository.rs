use super::{file_metadata::FileMetadata, file_name::FileName, file_timestamp::FileTimestamp};
use anyhow::{bail, Result};
use r2d2::PooledConnection;
use redis::{Client, Commands};
use std::collections::HashMap;
use ulid::Ulid;

fn get_key(id: String) -> String {
  format!("file-metadata:{}", id)
}

fn get_name_index_key(name: String) -> String {
  format!("file-metadata:name:{}", name)
}

impl From<HashMap<String, String>> for FileMetadata {
  fn from(values: HashMap<String, String>) -> Self {
    let id = values
      .get("id")
      .expect("id not found")
      .parse::<Ulid>()
      .expect("invalid id");

    let name = FileName(values.get("name").expect("name not found").to_string());

    let last_saved_at: FileTimestamp = values
      .get("last_saved_at")
      .expect("last_saved_at not found")
      .parse()
      .expect("invalid last_saved_at");

    Self {
      id,
      name,
      last_saved_at,
    }
  }
}

impl Into<Vec<(String, String)>> for FileMetadata {
  fn into(self) -> Vec<(String, String)> {
    vec![
      ("id".to_string(), self.id.to_string()),
      ("name".to_string(), self.name.0),
      ("last_saved_at".to_string(), self.last_saved_at.to_string()),
    ]
  }
}

pub struct FileMetadataRepository {
  redis_connection: PooledConnection<Client>,
}

impl FileMetadataRepository {
  pub fn new(redis_connection: PooledConnection<Client>) -> Self {
    Self { redis_connection }
  }

  pub fn find_by_id(&mut self, id: &str) -> Result<Option<FileMetadata>> {
    let res: HashMap<String, String> = self.redis_connection.hgetall(get_key(id.to_string()))?;

    if res.is_empty() {
      return Ok(None);
    } else {
      return Ok(Some(res.into()));
    }
  }

  pub fn find_by_name(&mut self, name: &FileName) -> Result<Option<FileMetadata>> {
    let id: Option<String> = self
      .redis_connection
      .get(get_name_index_key(name.to_string()))?;

    match id {
      Some(id) => {
        let res: HashMap<String, String> = self.redis_connection.hgetall(get_key(id))?;

        if res.is_empty() {
          return Ok(None);
        } else {
          return Ok(Some(res.into()));
        }
      }
      None => Ok(None),
    }
  }

  pub fn insert(&mut self, name: &FileName) -> Result<FileMetadata> {
    if self.find_by_name(name)?.is_some() {
      bail!("File already exists");
    }

    let file_metadata = FileMetadata {
      id: Ulid::new(),
      name: FileName::try_from(name.to_string())?,
      last_saved_at: FileTimestamp::now(),
    };

    let hset_items: Vec<(String, String)> = file_metadata.clone().try_into()?;
    redis::pipe()
      .atomic()
      .hset_multiple(get_key(file_metadata.id.into()), &hset_items)
      .ignore()
      .set(
        get_name_index_key(file_metadata.name.clone().into()),
        file_metadata.id.to_string(),
      )
      .ignore()
      .query(&mut self.redis_connection)?;

    Ok(file_metadata)
  }

  pub fn upsert(&mut self, name: &FileName) -> Result<FileMetadata> {
    match self.find_by_name(name)? {
      Some(file_metadata) => {
        let last_saved_at = FileTimestamp::now();

        redis::pipe()
          .atomic()
          .hset(
            get_key(file_metadata.id.into()),
            "last_saved_at",
            last_saved_at.to_string(),
          )
          .ignore()
          .query(&mut self.redis_connection)?;

        Ok(FileMetadata {
          id: file_metadata.id,
          name: file_metadata.name,
          last_saved_at,
        })
      }
      None => self.insert(name),
    }
  }
}
