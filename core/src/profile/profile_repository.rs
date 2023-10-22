use super::profile::{Profile, ProfileId};
use crate::files::file_metadata::file_name::FileName;
use anyhow::{bail, Error, Result};
use chrono::Utc;
use futures::future::join_all;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::{GenericCommands, JsonCommands, JsonGetOptions, SetCondition},
};
use std::sync::Arc;
use tracing::warn;

pub struct ProfileRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

impl ProfileRepository {
  pub fn key(&self, id: &ProfileId) -> String {
    format!("profile:{}", id.to_string())
  }

  pub fn profile_album_path(&self, album_file_name: &FileName) -> String {
    format!("$.albums.{}", album_file_name.to_string())
  }

  pub async fn find(&self, id: &ProfileId) -> Result<Option<Profile>> {
    let connection = self.redis_connection_pool.get().await?;
    let json: Option<String> = connection
      .json_get(self.key(id), JsonGetOptions::default())
      .await?;
    Ok(json.map(|json| serde_json::from_str(&json).unwrap()))
  }

  pub async fn get(&self, id: &ProfileId) -> Result<Profile> {
    let profile = self.find(id).await?;
    match profile {
      Some(profile) => Ok(profile),
      None => bail!("Profile does not exist"),
    }
  }

  pub async fn get_all(&self) -> Result<Vec<Profile>> {
    let connection = self.redis_connection_pool.get().await?;
    let keys: Vec<String> = connection.keys("profile:*").await?;
    let futures = keys.into_iter().map(|key| async {
      let json: String = connection.json_get(key, JsonGetOptions::default()).await?;
      Ok::<Profile, Error>(serde_json::from_str(&json).unwrap())
    });
    let profiles = join_all(futures)
      .await
      .into_iter()
      .filter_map(|profile: Result<Profile>| match profile {
        Ok(profile) => Some(profile),
        Err(_) => {
          warn!("Failed to deserialize profile");
          None
        }
      })
      .collect();
    Ok(profiles)
  }

  pub async fn exists(&self, id: &ProfileId) -> Result<bool> {
    let connection = self.redis_connection_pool.get().await?;
    let result: usize = connection.exists(self.key(id)).await?;
    Ok(result == 1)
  }

  pub async fn insert(&self, id: ProfileId, name: String) -> Result<Profile> {
    if self.exists(&id).await? {
      bail!("Profile already exists")
    }
    let profile = Profile {
      id: id.clone(),
      name,
      last_updated_at: Utc::now().naive_utc(),
      albums: Default::default(),
    };
    self
      .redis_connection_pool
      .get()
      .await?
      .json_set(
        self.key(&profile.id),
        "$",
        serde_json::to_string(&profile)?,
        SetCondition::default(),
      )
      .await?;
    Ok(profile)
  }

  pub async fn delete(&self, id: &ProfileId) -> Result<()> {
    if !self.exists(id).await? {
      bail!("Profile does not exist")
    }
    let connection = self.redis_connection_pool.get().await?;
    connection.del(self.key(id)).await?;
    Ok(())
  }

  pub async fn is_album_on_profile(
    &self,
    id: &ProfileId,
    album_file_name: &FileName,
  ) -> Result<bool> {
    if !self.exists(id).await? {
      bail!("Profile does not exist")
    }
    let connection = self.redis_connection_pool.get().await?;
    let json: Option<String> = connection
      .json_get(
        self.key(id),
        JsonGetOptions::default().path(self.profile_album_path(album_file_name)),
      )
      .await?;
    Ok(json.is_some() && json.unwrap() != "[]")
  }

  pub async fn put_album_on_profile(
    &self,
    id: &ProfileId,
    album_file_name: &FileName,
    factor: u32,
  ) -> Result<(Profile, bool)> {
    if !self.exists(id).await? {
      bail!("Profile does not exist")
    }
    let new_addition = !self.is_album_on_profile(id, album_file_name).await?;
    let connection = self.redis_connection_pool.get().await?;
    connection
      .json_set(
        self.key(id),
        self.profile_album_path(album_file_name),
        factor,
        SetCondition::default(),
      )
      .await?;
    Ok((self.get(id).await?, new_addition))
  }

  pub async fn remove_album_from_profile(
    &self,
    id: &ProfileId,
    album_file_name: &FileName,
  ) -> Result<()> {
    if !self.exists(id).await? {
      bail!("Profile does not exist")
    }
    let connection = self.redis_connection_pool.get().await?;
    connection
      .json_del(self.key(id), self.profile_album_path(album_file_name))
      .await?;
    Ok(())
  }
}
