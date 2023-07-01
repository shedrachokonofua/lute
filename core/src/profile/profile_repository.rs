use super::profile::{Profile, ProfileId};
use crate::files::file_metadata::file_name::FileName;
use anyhow::{bail, Result};
use chrono::Utc;
use rustis::{
  bb8::Pool,
  client::PooledClientManager,
  commands::SetCondition,
  commands::{JsonCommands, JsonGetOptions},
};
use std::sync::Arc;

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

  pub async fn does_profile_exist(&self, id: &ProfileId) -> Result<bool> {
    let profile = self.find(id).await?;
    Ok(profile.is_some())
  }

  pub async fn insert(&self, id: ProfileId, name: String) -> Result<Profile> {
    if self.does_profile_exist(&id).await? {
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

  pub async fn is_album_on_profile(
    &self,
    id: &ProfileId,
    album_file_name: &FileName,
  ) -> Result<bool> {
    if !self.does_profile_exist(id).await? {
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

  pub async fn add_album_to_profile(
    &self,
    id: &ProfileId,
    album_file_name: &FileName,
    factor: u32,
  ) -> Result<Profile> {
    if !self.does_profile_exist(id).await? {
      bail!("Profile does not exist")
    }
    let connection = self.redis_connection_pool.get().await?;
    connection
      .json_set(
        self.key(id),
        self.profile_album_path(album_file_name),
        factor,
        SetCondition::default(),
      )
      .await?;
    Ok(self.get(id).await?)
  }
}
