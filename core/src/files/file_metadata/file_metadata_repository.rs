use super::{file_metadata::FileMetadata, file_name::FileName, file_timestamp::FileTimestamp};
use anyhow::{bail, Result};
use r2d2::Pool;
use redis::{Client, Commands};
use std::{collections::HashMap, sync::Arc};
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

impl From<FileMetadata> for Vec<(String, String)> {
  fn from(val: FileMetadata) -> Self {
    vec![
      ("id".to_string(), val.id.to_string()),
      ("name".to_string(), val.name.0),
      ("last_saved_at".to_string(), val.last_saved_at.to_string()),
    ]
  }
}

pub struct FileMetadataRepository {
  pub redis_connection_pool: Arc<Pool<Client>>,
}

impl FileMetadataRepository {
  pub fn find_by_id(&self, id: &str) -> Result<Option<FileMetadata>> {
    let res: HashMap<String, String> = self
      .redis_connection_pool
      .get()?
      .hgetall(get_key(id.to_string()))?;

    if res.is_empty() {
      Ok(None)
    } else {
      Ok(Some(res.into()))
    }
  }

  pub fn find_by_name(&self, name: &FileName) -> Result<Option<FileMetadata>> {
    let id: Option<String> = self
      .redis_connection_pool
      .get()?
      .get(get_name_index_key(name.to_string()))?;

    match id {
      Some(id) => {
        let res: HashMap<String, String> =
          self.redis_connection_pool.get()?.hgetall(get_key(id))?;

        if res.is_empty() {
          Ok(None)
        } else {
          Ok(Some(res.into()))
        }
      }
      None => Ok(None),
    }
  }

  pub fn insert(&self, name: &FileName) -> Result<FileMetadata> {
    if self.find_by_name(name)?.is_some() {
      bail!("File already exists");
    }

    let file_metadata = FileMetadata {
      id: Ulid::new(),
      name: FileName::try_from(name.to_string())?,
      last_saved_at: FileTimestamp::now(),
    };

    let hset_items: Vec<(String, String)> = file_metadata.clone().try_into()?;
    let mut connection = self.redis_connection_pool.get()?;

    redis::pipe()
      .atomic()
      .hset_multiple(get_key(file_metadata.id.into()), &hset_items)
      .ignore()
      .set(
        get_name_index_key(file_metadata.name.clone().into()),
        file_metadata.id.to_string(),
      )
      .ignore()
      .query(&mut connection)?;

    Ok(file_metadata)
  }

  pub fn upsert(&self, name: &FileName) -> Result<FileMetadata> {
    let mut connection = self.redis_connection_pool.get()?;

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
          .query(&mut connection)?;

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
