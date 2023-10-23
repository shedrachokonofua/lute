use super::{file_metadata::FileMetadata, file_name::FileName, file_timestamp::FileTimestamp};
use anyhow::{bail, Result};
use rustis::{
  bb8::Pool,
  client::{BatchPreparedCommand, PooledClientManager},
  commands::{GenericCommands, HashCommands, StringCommands},
};
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

impl From<FileMetadata> for HashMap<String, String> {
  fn from(val: FileMetadata) -> Self {
    vec![
      ("id".to_string(), val.id.to_string()),
      ("name".to_string(), val.name.0),
      ("last_saved_at".to_string(), val.last_saved_at.to_string()),
    ]
    .into_iter()
    .collect()
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

#[derive(Debug, Clone)]
pub struct FileMetadataRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl FileMetadataRepository {
  pub async fn find_by_id(&self, id: &str) -> Result<Option<FileMetadata>> {
    let res: HashMap<String, String> = self
      .redis_connection_pool
      .get()
      .await?
      .hgetall(get_key(id.to_string()))
      .await?;

    if res.is_empty() {
      Ok(None)
    } else {
      Ok(Some(res.into()))
    }
  }

  pub async fn find_by_name(&self, name: &FileName) -> Result<Option<FileMetadata>> {
    let id: Option<String> = self
      .redis_connection_pool
      .get()
      .await?
      .get(get_name_index_key(name.to_string()))
      .await?;

    match id {
      Some(id) => {
        let res: HashMap<String, String> = self
          .redis_connection_pool
          .get()
          .await?
          .hgetall(get_key(id))
          .await?;

        if res.is_empty() {
          Ok(None)
        } else {
          Ok(Some(res.into()))
        }
      }
      None => Ok(None),
    }
  }

  pub async fn insert(&self, name: &FileName) -> Result<FileMetadata> {
    if self.find_by_name(name).await?.is_some() {
      bail!("File already exists");
    }

    let file_metadata = FileMetadata {
      id: Ulid::new(),
      name: FileName::try_from(name.to_string())?,
      last_saved_at: FileTimestamp::now(),
    };

    let hset_items: HashMap<String, String> = file_metadata.clone().try_into()?;
    let connection = self.redis_connection_pool.get().await?;

    let mut transaction = connection.create_transaction();
    transaction
      .hset(get_key(file_metadata.id.into()), hset_items)
      .forget();
    transaction
      .set(
        get_name_index_key(file_metadata.name.clone().into()),
        file_metadata.id.to_string(),
      )
      .queue();
    transaction.execute().await?;

    Ok(file_metadata)
  }

  pub async fn upsert(&self, name: &FileName) -> Result<FileMetadata> {
    let connection = self.redis_connection_pool.get().await?;

    match self.find_by_name(name).await? {
      Some(file_metadata) => {
        let last_saved_at = FileTimestamp::now();
        connection
          .hset(
            get_key(file_metadata.id.into()),
            ("last_saved_at", last_saved_at.to_string()),
          )
          .await?;

        Ok(FileMetadata {
          id: file_metadata.id,
          name: file_metadata.name,
          last_saved_at,
        })
      }
      None => self.insert(name).await,
    }
  }

  pub async fn delete(&self, name: &FileName) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection.del(get_name_index_key(name.to_string())).await?;
    let id: Option<String> = connection.get(get_name_index_key(name.to_string())).await?;
    if let Some(id) = id {
      connection.del(get_key(id)).await?;
    }

    Ok(())
  }
}
