use super::{
  profile::{Profile, ProfileId},
  profile_repository::ProfileRepository,
  profile_summary::ProfileSummary,
};
use crate::{
  albums::album_read_model_repository::AlbumReadModelRepository,
  events::{
    event::{Event, EventPayload, Stream},
    event_publisher::EventPublisher,
  },
  files::file_metadata::file_name::FileName,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub struct ProfileInteractor {
  profile_repository: ProfileRepository,
  album_read_model_repository: AlbumReadModelRepository,
  event_publisher: EventPublisher,
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
      event_publisher: EventPublisher::new(redis_connection_pool),
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
    file_name: &FileName,
    factor: u32,
  ) -> Result<Profile> {
    if !self.album_read_model_repository.exists(file_name).await? {
      anyhow::bail!("Album does not exist");
    }

    let (profile, new_addition) = self
      .profile_repository
      .add_album_to_profile(id, file_name, factor)
      .await?;

    if new_addition {
      self
        .event_publisher
        .publish(
          Stream::Profile,
          EventPayload::from_event(Event::ProfileAlbumAdded {
            profile_id: id.clone(),
            file_name: file_name.clone(),
            factor,
          }),
        )
        .await?;
    }

    Ok(profile)
  }

  pub async fn add_many_albums_to_profile(
    &self,
    id: &ProfileId,
    entries: Vec<(FileName, u32)>,
  ) -> Result<Profile> {
    for (file_name, factor) in entries {
      self.add_album_to_profile(id, &file_name, factor).await?;
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
