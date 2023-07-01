use super::{
  profile::{Profile, ProfileId},
  profile_repository::ProfileRepository,
  profile_summary::ProfileSummary,
};
use crate::{
  albums::album_read_model_repository::AlbumReadModelRepository,
  files::file_metadata::file_name::FileName,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub struct ProfileInteractor {
  profile_repository: ProfileRepository,
  album_read_model_repository: AlbumReadModelRepository,
}

impl ProfileInteractor {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      profile_repository: ProfileRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      album_read_model_repository: AlbumReadModelRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
    }
  }

  pub async fn create_profile(&self, id: ProfileId, name: String) -> Result<Profile> {
    let profile = self.profile_repository.insert(id, name).await?;
    Ok(profile)
  }

  pub async fn get_profile(&self, id: &ProfileId) -> Result<Profile> {
    self.profile_repository.get(id).await
  }

  pub async fn add_album_to_profile(
    &self,
    id: &ProfileId,
    album_file_name: &FileName,
    factor: u32,
  ) -> Result<Profile> {
    let _ = self
      .album_read_model_repository
      .get(album_file_name)
      .await?;

    self
      .profile_repository
      .add_album_to_profile(id, album_file_name, factor)
      .await
  }

  pub async fn add_many_albums_to_profile(
    &self,
    id: &ProfileId,
    entries: Vec<(FileName, u32)>,
  ) -> Result<Profile> {
    for (album_file_name, factor) in entries {
      self
        .add_album_to_profile(id, &album_file_name, factor)
        .await?;
    }
    Ok(self.get_profile(id).await?)
  }

  pub async fn get_profile_summary(&self, id: &ProfileId) -> Result<ProfileSummary> {
    let profile = self.profile_repository.get(id).await?;
    let albums = self
      .album_read_model_repository
      .get_many(
        profile
          .albums
          .iter()
          .map(|(file_name, _)| file_name.clone())
          .collect(),
      )
      .await?;
    Ok(profile.summarize(albums))
  }
}
