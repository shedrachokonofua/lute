use super::{
  profile::{Profile, ProfileId},
  profile_interactor::ProfileInteractor,
  profile_summary::{ItemWithFactor, ProfileSummary},
};
use crate::{
  files::file_metadata::file_name::FileName,
  proto::{
    self, AddManyAlbumsToProfileReply, AddManyAlbumsToProfileRequest, CreateProfileReply,
    CreateProfileRequest, GetProfileReply, GetProfileRequest, GetProfileSummaryReply,
    GetProfileSummaryRequest, ImportSavedSpotifyTracksRequest,
  },
  settings::Settings,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tonic::{Request, Response, Status};

impl From<Profile> for proto::Profile {
  fn from(val: Profile) -> Self {
    proto::Profile {
      id: val.id.to_string(),
      name: val.name.clone(),
      last_updated_at: val.last_updated_at.to_string(),
      albums: val
        .albums
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect(),
    }
  }
}

impl From<ItemWithFactor> for proto::ItemWithFactor {
  fn from(val: ItemWithFactor) -> Self {
    proto::ItemWithFactor {
      item: val.item,
      factor: val.factor,
    }
  }
}

impl From<ProfileSummary> for proto::ProfileSummary {
  fn from(val: ProfileSummary) -> Self {
    proto::ProfileSummary {
      id: val.id.to_string(),
      name: val.name,
      album_count: val.album_count,
      indexed_album_count: val.indexed_album_count,
      average_rating: val.average_rating,
      median_year: val.median_year,
      artists: val.artists.into_iter().map(Into::into).collect(),
      primary_genres: val.primary_genres.into_iter().map(Into::into).collect(),
      secondary_genres: val.secondary_genres.into_iter().map(Into::into).collect(),
      descriptors: val.descriptors.into_iter().map(Into::into).collect(),
      years: val.years.into_iter().map(Into::into).collect(),
      decades: val.decades.into_iter().map(Into::into).collect(),
    }
  }
}

pub struct ProfileService {
  profile_interactor: ProfileInteractor,
}

impl ProfileService {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
  ) -> Self {
    Self {
      profile_interactor: ProfileInteractor::new(settings, redis_connection_pool),
    }
  }
}

#[tonic::async_trait]
impl proto::ProfileService for ProfileService {
  async fn create_profile(
    &self,
    request: Request<CreateProfileRequest>,
  ) -> Result<Response<CreateProfileReply>, Status> {
    let request = request.into_inner();
    let id: ProfileId = request.id.try_into().map_err(|err| {
      tracing::error!("invalid profile id: {:?}", err);
      Status::invalid_argument("invalid profile id")
    })?;
    let profile = self
      .profile_interactor
      .create_profile(id, request.name)
      .await
      .map_err(|err| {
        tracing::error!("failed to create profile: {:?}", err);
        Status::internal("failed to create profile")
      })?;
    let reply = CreateProfileReply {
      profile: Some(profile.into()),
    };
    Ok(Response::new(reply))
  }

  async fn get_profile(
    &self,
    request: Request<GetProfileRequest>,
  ) -> Result<Response<GetProfileReply>, Status> {
    let request = request.into_inner();
    let id: ProfileId = request.id.try_into().map_err(|err| {
      tracing::error!("invalid profile id: {:?}", err);
      Status::invalid_argument("invalid profile id")
    })?;
    let profile = self
      .profile_interactor
      .get_profile(&id)
      .await
      .map_err(|err| {
        tracing::error!("failed to get profile: {:?}", err);
        Status::internal("failed to get profile")
      })?;
    let reply = GetProfileReply {
      profile: Some(profile.into()),
    };
    Ok(Response::new(reply))
  }

  async fn get_profile_summary(
    &self,
    request: Request<GetProfileSummaryRequest>,
  ) -> Result<Response<GetProfileSummaryReply>, Status> {
    let request = request.into_inner();
    let id: ProfileId = request.id.try_into().map_err(|err| {
      tracing::error!("invalid profile id: {:?}", err);
      Status::invalid_argument("invalid profile id")
    })?;
    let profile_summary = self
      .profile_interactor
      .get_profile_summary(&id)
      .await
      .map_err(|err| {
        tracing::error!("failed to get profile summary: {:?}", err);
        Status::internal("failed to get profile summary")
      })?;
    let reply = GetProfileSummaryReply {
      summary: Some(profile_summary.into()),
    };
    Ok(Response::new(reply))
  }

  async fn add_many_albums_to_profile(
    &self,
    request: Request<AddManyAlbumsToProfileRequest>,
  ) -> Result<Response<AddManyAlbumsToProfileReply>, Status> {
    let request = request.into_inner();
    let id: ProfileId = request.profile_id.try_into().map_err(|err| {
      tracing::error!("invalid profile id: {:?}", err);
      Status::invalid_argument("invalid profile id")
    })?;
    let entries = request
      .albums
      .into_iter()
      .map(|entry| -> (FileName, u32) {
        let album_file_name: FileName = entry
          .file_name
          .try_into()
          .map_err(|err| {
            tracing::error!("invalid album file name: {:?}", err);
            Status::invalid_argument("invalid album file name")
          })
          .unwrap();
        (album_file_name, entry.factor)
      })
      .collect();
    let profile = self
      .profile_interactor
      .add_many_albums_to_profile(&id, entries)
      .await
      .map_err(|err| {
        tracing::error!("failed to add album to profile: {:?}", err);
        Status::internal("failed to add album to profile")
      })?;
    let reply = AddManyAlbumsToProfileReply {
      profile: Some(profile.into()),
    };
    Ok(Response::new(reply))
  }

  async fn import_saved_spotify_tracks(
    &self,
    request: Request<ImportSavedSpotifyTracksRequest>,
  ) -> Result<Response<()>, Status> {
    let profile_id = ProfileId::try_from(request.into_inner().profile_id).map_err(|err| {
      tracing::error!("invalid profile id: {:?}", err);
      Status::invalid_argument("invalid profile id")
    })?;
    self
      .profile_interactor
      .import_saved_spotify_tracks(&profile_id)
      .await
      .map_err(|err| {
        tracing::error!("failed to import saved spotify tracks: {:?}", err);
        Status::internal("failed to import saved spotify tracks")
      })?;

    Ok(Response::new(()))
  }
}
