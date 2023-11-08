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
  albums::{album_read_model::AlbumReadModel, album_repository::AlbumRepository},
  events::{
    event::{Event, EventPayload, Stream},
    event_publisher::EventPublisher,
  },
  files::file_metadata::file_name::FileName,
  lookup::{
    album_search_lookup::{AlbumSearchLookup, AlbumSearchLookupQuery, AlbumSearchLookupStatus},
    lookup_interactor::LookupInteractor,
  },
  settings::Settings,
  spotify::spotify_client::{SpotifyClient, SpotifyTrack},
};
use anyhow::Result;
use futures::future::join_all;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tracing::warn;

pub struct PendingSpotifyImport {
  pub profile_id: ProfileId,
  pub factor: u32,
  pub album_search_lookup: AlbumSearchLookup,
}

pub struct ProfileInteractor {
  profile_repository: ProfileRepository,
  album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  event_publisher: EventPublisher,
  spotify_client: SpotifyClient,
  lookup_interactor: LookupInteractor,
  spotify_import_repository: SpotifyImportRepository,
}

impl ProfileInteractor {
  pub fn new(
    settings: Arc<Settings>,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    sqlite_connection: Arc<tokio_rusqlite::Connection>,
    album_repository: Arc<dyn AlbumRepository + Send + Sync + 'static>,
  ) -> Self {
    Self {
      profile_repository: ProfileRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      album_repository: Arc::clone(&album_repository),
      event_publisher: EventPublisher::new(Arc::clone(&settings), Arc::clone(&sqlite_connection)),
      spotify_client: SpotifyClient::new(&settings.spotify, Arc::clone(&redis_connection_pool)),
      lookup_interactor: LookupInteractor::new(
        Arc::clone(&settings),
        Arc::clone(&redis_connection_pool),
        Arc::clone(&sqlite_connection),
      ),
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

  pub async fn get_all_profiles(&self) -> Result<Vec<Profile>> {
    self.profile_repository.get_all().await
  }

  pub async fn put_album_on_profile(
    &self,
    id: &ProfileId,
    file_name: &FileName,
    factor: u32,
  ) -> Result<Profile> {
    let album = self.album_repository.get(file_name).await?;
    let file_name_to_add = album.duplicate_of.unwrap_or(file_name.clone());
    let (profile, new_addition) = self
      .profile_repository
      .put_album_on_profile(id, &file_name_to_add, factor)
      .await
      .map_err(|e| {
        anyhow::anyhow!(
          "Failed to add album {} to profile: {}",
          file_name_to_add.to_string(),
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

  pub async fn put_many_albums_on_profile(
    &self,
    id: &ProfileId,
    entries: Vec<(FileName, u32)>,
  ) -> Result<Profile> {
    for (file_name, factor) in entries {
      match self.put_album_on_profile(id, &file_name, factor).await {
        Ok(_) => (),
        Err(e) => {
          warn!(
            profile_id = id.to_string(),
            file_name = file_name.to_string(),
            factor = factor,
            error = e.to_string(),
            "Failed to add album to profile"
          );
        }
      }
    }
    self.get_profile(id).await
  }

  pub async fn remove_album_from_profile(
    &self,
    id: &ProfileId,
    file_name: &FileName,
  ) -> Result<()> {
    self
      .profile_repository
      .remove_album_from_profile(id, file_name)
      .await
  }

  pub async fn get_profile_summary_and_albums(
    &self,
    id: &ProfileId,
  ) -> Result<(ProfileSummary, Vec<AlbumReadModel>)> {
    let profile = self.profile_repository.get(id).await?;
    let albums = if !profile.albums.is_empty() {
      self
        .album_repository
        .get_many(profile.albums.keys().cloned().collect())
        .await?
    } else {
      vec![]
    };
    Ok((profile.summarize(&albums), albums))
  }

  pub async fn get_profile_summary(&self, id: &ProfileId) -> Result<ProfileSummary> {
    let (profile_summary, _) = self.get_profile_summary_and_albums(id).await?;
    Ok(profile_summary)
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
          id,
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
      .put_many_albums_on_profile(
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
        .delete_subscription(id, &subscription.album_search_lookup_query)
    }))
    .await;

    Ok(())
  }

  pub async fn import_saved_spotify_tracks(&self, id: &ProfileId) -> Result<()> {
    let spotify_tracks = self.spotify_client.get_saved_tracks().await?;
    self.import_spotify_tracks(id, spotify_tracks).await
  }

  pub async fn import_spotify_playlist_tracks(
    &self,
    id: &ProfileId,
    playlist_id: &str,
  ) -> Result<()> {
    let spotify_tracks = self.spotify_client.get_playlist_tracks(playlist_id).await?;
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

  pub async fn delete_spotify_import_subscription(
    &self,
    profile_id: &ProfileId,
    album_search_lookup_query: &AlbumSearchLookupQuery,
  ) -> Result<()> {
    self
      .spotify_import_repository
      .delete_subscription(profile_id, album_search_lookup_query)
      .await
  }

  pub async fn get_pending_spotify_imports(
    &self,
    profile_id: &ProfileId,
  ) -> Result<Vec<PendingSpotifyImport>> {
    let subscriptions = self
      .spotify_import_repository
      .find_subscriptions_by_profile_id(profile_id)
      .await?;

    if subscriptions.is_empty() {
      return Ok(Vec::new());
    }

    let lookups = self
      .lookup_interactor
      .find_many_album_search_lookups(
        subscriptions
          .iter()
          .map(|subscription| &subscription.album_search_lookup_query)
          .collect(),
      )
      .await?
      .into_iter()
      .flatten()
      .filter(|lookup| lookup.status() != AlbumSearchLookupStatus::AlbumParsed)
      .collect::<Vec<_>>();
    let pending_imports: Vec<PendingSpotifyImport> = lookups
      .into_iter()
      .map(|lookup| {
        let subscription = subscriptions
          .iter()
          .find(|subscription| &subscription.album_search_lookup_query == lookup.query())
          .unwrap();
        PendingSpotifyImport {
          profile_id: profile_id.clone(),
          factor: subscription.factor,
          album_search_lookup: lookup,
        }
      })
      .collect::<Vec<_>>();
    Ok(pending_imports)
  }

  pub async fn delete_profile(&self, id: &ProfileId) -> Result<()> {
    self.profile_repository.delete(id).await
  }

  pub async fn clear_pending_spotify_imports(&self, profile_id: &ProfileId) -> Result<()> {
    let pending_imports = self.get_pending_spotify_imports(profile_id).await?;
    for pending_import in pending_imports {
      self
        .delete_spotify_import_subscription(
          &pending_import.profile_id,
          pending_import.album_search_lookup.query(),
        )
        .await?;
      self
        .lookup_interactor
        .delete_album_search_lookup(pending_import.album_search_lookup.query())
        .await?;
    }
    Ok(())
  }
}
