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
  albums::{album_interactor::AlbumInteractor, album_read_model::AlbumReadModel},
  events::{
    event::{Event, EventPayloadBuilder, Topic},
    event_publisher::EventPublisher,
  },
  files::file_metadata::file_name::FileName,
  helpers::document_store::DocumentStore,
  lookup::{
    AlbumSearchLookup, AlbumSearchLookupDiscriminants, AlbumSearchLookupQuery, LookupInteractor,
  },
  spotify::spotify_client::{SpotifyClient, SpotifyTrack},
};
use anyhow::Result;
use futures::future::join_all;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::{collections::HashMap, sync::Arc};
use tracing::{instrument, warn};

pub struct PendingSpotifyImport {
  pub profile_id: ProfileId,
  pub factor: u32,
  pub album_search_lookup: AlbumSearchLookup,
}

pub struct ProfileInteractor {
  profile_repository: ProfileRepository,
  album_interactor: Arc<AlbumInteractor>,
  event_publisher: Arc<EventPublisher>,
  spotify_client: Arc<SpotifyClient>,
  lookup_interactor: Arc<LookupInteractor>,
  spotify_import_repository: SpotifyImportRepository,
}

impl ProfileInteractor {
  pub fn new(
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
    event_publisher: Arc<EventPublisher>,
    album_interactor: Arc<AlbumInteractor>,
    lookup_interactor: Arc<LookupInteractor>,
    spotify_client: Arc<SpotifyClient>,
    doc_store: Arc<DocumentStore>,
  ) -> Self {
    Self {
      profile_repository: ProfileRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      album_interactor,
      event_publisher,
      spotify_client,
      lookup_interactor,
      spotify_import_repository: SpotifyImportRepository::new(Arc::clone(&doc_store)),
    }
  }

  pub async fn create_profile(&self, id: ProfileId, name: String) -> Result<Profile> {
    let profile = self.profile_repository.insert(id, name).await?;
    Ok(profile)
  }

  pub async fn get_profile(&self, id: &ProfileId) -> Result<Profile> {
    self.profile_repository.get(id).await
  }

  pub async fn find_profile(&self, id: &ProfileId) -> Result<Option<Profile>> {
    self.profile_repository.find(id).await
  }

  pub async fn get_all_profiles(&self) -> Result<Vec<Profile>> {
    self.profile_repository.get_all().await
  }

  async fn put_album_on_profile_with_model(
    &self,
    id: &ProfileId,
    album: AlbumReadModel,
    factor: u32,
  ) -> Result<Profile> {
    let file_name = album.duplicate_of.unwrap_or(album.file_name.clone());
    let (profile, new_addition) = self
      .profile_repository
      .put_album_on_profile(id, &file_name, factor)
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
          Topic::Profile,
          EventPayloadBuilder::default()
            .key(format!("{}:{}", id.to_string(), file_name.to_string()))
            .event(Event::ProfileAlbumAdded {
              profile_id: id.clone(),
              file_name,
              factor,
            })
            .build()?,
        )
        .await?;
    }

    Ok(profile)
  }

  pub async fn put_album_on_profile(
    &self,
    id: &ProfileId,
    file_name: &FileName,
    factor: u32,
  ) -> Result<Profile> {
    let album = self.album_interactor.get(file_name).await?;
    self
      .put_album_on_profile_with_model(id, album, factor)
      .await
  }

  pub async fn put_many_albums_on_profile(
    &self,
    id: &ProfileId,
    entries: Vec<(FileName, u32)>,
  ) -> Result<Profile> {
    let mut albums = self
      .album_interactor
      .find_many(
        entries
          .iter()
          .map(|(file_name, _)| file_name.clone())
          .collect(),
      )
      .await?;
    for (file_name, factor) in entries {
      if let Some(album) = albums.remove(&file_name) {
        self
          .put_album_on_profile_with_model(id, album, factor)
          .await?;
      } else {
        warn!(
          profile_id = id.to_string(),
          file_name = file_name.to_string(),
          factor = factor,
          "Could not find album to add to profile"
        );
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
        .album_interactor
        .find_many(profile.albums.keys().cloned().collect())
        .await?
        .drain()
        .map(|(_, album)| album)
        .collect()
    } else {
      vec![]
    };
    Ok((profile.summarize(&albums), albums))
  }

  #[instrument(skip(self))]
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
      .filter(|(lookup, _)| lookup.status() == AlbumSearchLookupDiscriminants::AlbumParsed)
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
      .filter(|(_, lookup)| lookup.status() != AlbumSearchLookupDiscriminants::AlbumParsed)
      .collect::<HashMap<_, _>>();

    let pending_imports = subscriptions
      .into_iter()
      .filter_map(|subscription| {
        lookups
          .get(&subscription.album_search_lookup_query.to_encoded_string())
          .map(|lookup| PendingSpotifyImport {
            profile_id: profile_id.clone(),
            factor: subscription.factor,
            album_search_lookup: lookup.clone(),
          })
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
