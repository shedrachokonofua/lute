use super::{
  profile::{Profile, ProfileId},
  profile_repository::ProfileRepository,
  profile_summary::ProfileSummary,
  spotify_import_lookup_subscription::{
    build_spotify_import_lookup_subscriptions, SpotifyImportLookupSubscription,
  },
  spotify_import_repository::SpotifyImportRepository,
};
use crate::{
  albums::album_read_model_repository::AlbumReadModelRepository,
  events::{
    event::{Event, EventPayload, Stream},
    event_publisher::EventPublisher,
  },
  files::file_metadata::file_name::FileName,
  lookup::{
    album_search_lookup::{AlbumSearchLookupQuery, AlbumSearchLookupStatus},
    lookup_interactor::LookupInteractor,
  },
  settings::Settings,
  spotify::spotify_client::{SpotifyClient, SpotifyTrack},
};
use anyhow::Result;
use futures::future::join_all;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub struct ProfileInteractor {
  profile_repository: ProfileRepository,
  album_read_model_repository: AlbumReadModelRepository,
  event_publisher: EventPublisher,
  spotify_client: SpotifyClient,
  lookup_interactor: LookupInteractor,
  spotify_import_repository: SpotifyImportRepository,
}

impl ProfileInteractor {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
  ) -> Self {
    Self {
      profile_repository: ProfileRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      album_read_model_repository: AlbumReadModelRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      event_publisher: EventPublisher::new(Arc::clone(&redis_connection_pool)),
      spotify_client: SpotifyClient::new(&settings.spotify, Arc::clone(&redis_connection_pool)),
      lookup_interactor: LookupInteractor::new(Arc::clone(&redis_connection_pool)),
      spotify_import_repository: SpotifyImportRepository {
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
    file_name: &FileName,
    factor: u32,
  ) -> Result<Profile> {
    if !self.album_read_model_repository.exists(file_name).await? {
      anyhow::bail!("Album does not exist");
    }

    let (profile, new_addition) = self
      .profile_repository
      .add_album_to_profile(id, file_name, factor)
      .await
      .map_err(|e| {
        anyhow::anyhow!(
          "Failed to add album {} to profile: {}",
          file_name.to_string(),
          e
        )
      })?;

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
      match self.add_album_to_profile(id, &file_name, factor).await {
        Ok(_) => (),
        Err(e) => {
          tracing::warn!(
            profile_id = id.to_string(),
            file_name = file_name.to_string(),
            factor = factor,
            error = e.to_string(),
            "Failed to add album to profile"
          );
        }
      }
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

  async fn import_spotify_tracks(
    &self,
    id: &ProfileId,
    spotify_tracks: Vec<SpotifyTrack>,
  ) -> Result<()> {
    let subscriptions = build_spotify_import_lookup_subscriptions(id, spotify_tracks);
    join_all(subscriptions.iter().map(|subscription| async move {
      self
        .spotify_import_repository
        .put_subscription(
          &subscription.album_search_lookup_query,
          &id,
          subscription.factor,
        )
        .await
    }))
    .await;
    let pairs = join_all(subscriptions.iter().map(|subscription| async move {
      let lookup = self
        .lookup_interactor
        .search_album(
          subscription
            .album_search_lookup_query
            .artist_name()
            .to_string(),
          subscription
            .album_search_lookup_query
            .album_name()
            .to_string(),
        )
        .await
        .expect("failed to search album");
      (lookup, subscription)
    }))
    .await;
    let complete_pairs = pairs
      .into_iter()
      .filter(|(lookup, _)| lookup.status() == AlbumSearchLookupStatus::AlbumParsed)
      .collect::<Vec<_>>();
    self
      .add_many_albums_to_profile(
        id,
        complete_pairs
          .iter()
          .map(|(lookup, subscription)| {
            (
              lookup.parsed_album_search_result().unwrap().file_name,
              subscription.factor,
            )
          })
          .collect(),
      )
      .await?;
    join_all(complete_pairs.iter().map(|(_, subscription)| {
      self
        .spotify_import_repository
        .remove_subscription(&id, &subscription.album_search_lookup_query)
    }))
    .await;

    Ok(())
  }

  pub async fn import_saved_spotify_tracks(&self, id: &ProfileId) -> Result<()> {
    let spotify_tracks = self.spotify_client.get_saved_tracks().await?;
    self.import_spotify_tracks(id, spotify_tracks).await
  }

  pub async fn find_spotify_import_subscriptions_by_query(
    &self,
    album_search_lookup_query: &AlbumSearchLookupQuery,
  ) -> Result<Vec<SpotifyImportLookupSubscription>> {
    self
      .spotify_import_repository
      .find_subscriptions_by_query(album_search_lookup_query)
      .await
  }

  pub async fn remove_spotify_import_subscription(
    &self,
    profile_id: &ProfileId,
    album_search_lookup_query: &AlbumSearchLookupQuery,
  ) -> Result<()> {
    self
      .spotify_import_repository
      .remove_subscription(profile_id, album_search_lookup_query)
      .await
  }
}
