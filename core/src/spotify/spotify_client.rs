use super::spotify_credential_repository::{
  SpotifyCredentialRepository, SpotifyCredentials, SCOPES,
};
use crate::{
  albums::album_read_model::AlbumReadModel, helpers::key_value_store::KeyValueStore, proto,
  settings::SpotifySettings,
};
use anyhow::{anyhow, Error, Result};
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use governor::{DefaultDirectRateLimiter, Jitter, Quota, RateLimiter};
use lazy_static::lazy_static;
use nonzero::nonzero;
use rspotify::{
  http::HttpError,
  model::{
    AlbumId, AudioFeatures, FullTrack, PlayableId, PlayableItem, PlaylistId, SavedTrack,
    SearchResult, SearchType, SimplifiedAlbum, SimplifiedArtist, SimplifiedTrack, TrackId,
  },
  prelude::{BaseClient, OAuthClient},
  AuthCodeSpotify, ClientError, Credentials, OAuth, Token,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use strsim::jaro_winkler;
use thiserror::Error;
use tokio::sync::mpsc::unbounded_channel;
use tracing::{debug, error, info, warn};
use unidecode::unidecode;

lazy_static! {
  static ref RATE_LIMITER: DefaultDirectRateLimiter = RateLimiter::direct(Quota::per_second(nonzero!(2u32))); // API limit is 180/min
}

#[derive(Error, Debug)]
pub enum SpotifyClientError {
  #[error("Spotify API rate limit exceeded.")]
  TooManyRequests(Option<usize>),
  #[error("Spotify client error: {0}")]
  Unknown(#[from] ClientError),
}

impl From<Token> for SpotifyCredentials {
  fn from(token: Token) -> Self {
    Self {
      access_token: token.access_token,
      refresh_token: token.refresh_token.unwrap(),
      expires_at: token.expires_at.unwrap().naive_utc(),
    }
  }
}

impl From<SpotifyCredentials> for Token {
  fn from(credentials: SpotifyCredentials) -> Self {
    let expires_at = DateTime::from_naive_utc_and_offset(credentials.expires_at, Utc);

    Self {
      scopes: SCOPES.clone(),
      access_token: credentials.access_token,
      refresh_token: Some(credentials.refresh_token),
      expires_at: Some(expires_at),
      expires_in: credentials
        .expires_at
        .signed_duration_since(Utc::now().naive_utc()),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotifyArtistReference {
  pub spotify_id: String,
  pub name: String,
}

impl Into<proto::SpotifyArtistReference> for SpotifyArtistReference {
  fn into(self) -> proto::SpotifyArtistReference {
    proto::SpotifyArtistReference {
      spotify_id: self.spotify_id,
      name: self.name,
    }
  }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum SpotifyAlbumType {
  Album,
  Single,
  Compilation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotifyTrackReference {
  pub spotify_id: String,
  pub name: String,
  pub artists: Vec<SpotifyArtistReference>,
}

impl Into<proto::SpotifyTrackReference> for SpotifyTrackReference {
  fn into(self) -> proto::SpotifyTrackReference {
    proto::SpotifyTrackReference {
      spotify_id: self.spotify_id,
      name: self.name,
      artists: self.artists.into_iter().map(Into::into).collect(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotifyAlbum {
  pub spotify_id: String,
  pub name: String,
  pub album_type: SpotifyAlbumType,
  pub artists: Vec<SpotifyArtistReference>,
  pub tracks: Vec<SpotifyTrackReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotifyAlbumReference {
  pub spotify_id: String,
  pub name: String,
  pub album_type: SpotifyAlbumType,
}

impl From<SpotifyAlbum> for SpotifyAlbumReference {
  fn from(album: SpotifyAlbum) -> Self {
    Self {
      spotify_id: album.spotify_id,
      name: album.name,
      album_type: album.album_type,
    }
  }
}

pub struct SpotifyTrack {
  pub spotify_id: String,
  pub name: String,
  pub artists: Vec<SpotifyArtistReference>,
  pub album: SpotifyAlbumReference,
}

fn get_spotify_album(
  simplified_album: SimplifiedAlbum,
  tracks: Vec<SimplifiedTrack>,
) -> SpotifyAlbum {
  SpotifyAlbum {
    spotify_id: simplified_album.id.unwrap().to_string(),
    name: simplified_album.name,
    album_type: match simplified_album.album_type.unwrap().as_str() {
      "album" => SpotifyAlbumType::Album,
      "single" => SpotifyAlbumType::Single,
      "compilation" => SpotifyAlbumType::Compilation,
      _ => panic!("Unknown album type"),
    },
    artists: simplified_album
      .artists
      .iter()
      .map(|artist| SpotifyArtistReference {
        spotify_id: artist.id.clone().expect("Artist ID is missing").to_string(),
        name: artist.name.clone(),
      })
      .collect(),
    tracks: tracks
      .iter()
      .map(|track| SpotifyTrackReference {
        spotify_id: track.id.clone().expect("Track ID is missing").to_string(),
        name: track.name.clone(),
        artists: track
          .artists
          .iter()
          .map(|artist| SpotifyArtistReference {
            spotify_id: artist.id.clone().expect("Artist ID is missing").to_string(),
            name: artist.name.clone(),
          })
          .collect(),
      })
      .collect(),
  }
}

impl TryFrom<SimplifiedAlbum> for SpotifyAlbumReference {
  type Error = Error;

  fn try_from(simplified_album: SimplifiedAlbum) -> Result<Self> {
    let spotify_id = simplified_album
      .id
      .ok_or_else(|| anyhow!("Album ID is missing"))?;

    let album_type = match simplified_album
      .album_type
      .ok_or_else(|| anyhow!("Album type is missing"))?
      .as_str()
    {
      "album" => SpotifyAlbumType::Album,
      "single" => SpotifyAlbumType::Single,
      "compilation" => SpotifyAlbumType::Compilation,
      _ => return Err(anyhow!("Unknown album type")),
    };

    Ok(SpotifyAlbumReference {
      spotify_id: spotify_id.to_string(),
      name: simplified_album.name,
      album_type,
    })
  }
}

impl TryFrom<&SimplifiedArtist> for SpotifyArtistReference {
  type Error = Error;

  fn try_from(simplified_artist: &SimplifiedArtist) -> Result<Self> {
    let spotify_id = simplified_artist
      .id
      .clone()
      .ok_or_else(|| anyhow!("Artist ID is missing"))?;

    Ok(SpotifyArtistReference {
      spotify_id: spotify_id.to_string(),
      name: simplified_artist.name.clone(),
    })
  }
}

impl TryFrom<SavedTrack> for SpotifyTrack {
  type Error = Error;

  fn try_from(saved_track: SavedTrack) -> Result<Self> {
    let spotify_id = saved_track
      .track
      .id
      .ok_or_else(|| anyhow!("Track ID is missing"))?;

    Ok(SpotifyTrack {
      spotify_id: spotify_id.to_string(),
      name: saved_track.track.name,
      artists: saved_track
        .track
        .artists
        .iter()
        .map(|artist| (artist, artist.try_into()))
        .filter_map(
          |(spotify_artist, artist): (&SimplifiedArtist, Result<SpotifyArtistReference>)| {
            match artist {
              Ok(artist) => Some(artist),
              Err(err) => {
                warn!(
                  err = err.to_string(),
                  spotify_artist = format!("{:?}", spotify_artist),
                  "Failed to convert artist"
                );
                None
              }
            }
          },
        )
        .collect(),
      album: saved_track.track.album.try_into()?,
    })
  }
}

impl TryFrom<FullTrack> for SpotifyTrack {
  type Error = anyhow::Error;

  fn try_from(full_track: FullTrack) -> Result<Self> {
    let spotify_id = full_track.id.ok_or_else(|| anyhow!("ID is missing"))?;

    Ok(SpotifyTrack {
      spotify_id: spotify_id.to_string(),
      name: full_track.name,
      artists: full_track
        .artists
        .iter()
        .map(|artist| (artist, artist.try_into()))
        .filter_map(
          |(spotify_artist, artist): (&SimplifiedArtist, Result<SpotifyArtistReference>)| {
            match artist {
              Ok(artist) => Some(artist),
              Err(err) => {
                warn!(
                  err = err.to_string(),
                  spotify_artist = format!("{:?}", spotify_artist),
                  "Failed to convert artist"
                );
                None
              }
            }
          },
        )
        .collect(),
      album: full_track.album.try_into()?,
    })
  }
}

fn get_features_embedding(features: AudioFeatures) -> Vec<f32> {
  vec![
    features.acousticness,
    features.danceability,
    features.energy,
    features.instrumentalness,
    features.liveness,
    features.loudness,
    features.speechiness,
    features.tempo,
    features.valence,
  ]
}

fn map_spotify_error(err: ClientError) -> SpotifyClientError {
  if let ClientError::Http(http_error) = &err {
    if let HttpError::StatusCode(response) = http_error.as_ref() {
      if response.status().as_u16() == 429 {
        let headers_string = format!("{:?}", response.headers());
        warn!(headers = headers_string, "Error response headers");
        let retry_after = response
          .headers()
          .get("Retry-After")
          .and_then(|retry_after| {
            retry_after
              .to_str()
              .ok()
              .and_then(|retry_after| retry_after.parse::<usize>().ok())
          });

        return SpotifyClientError::TooManyRequests(retry_after);
      }
    }
  }
  SpotifyClientError::Unknown(err)
}

pub struct SpotifyClient {
  pub settings: SpotifySettings,
  pub spotify_credential_repository: SpotifyCredentialRepository,
}

async fn get_client_token(client: &AuthCodeSpotify) -> Token {
  client.token.lock().await.unwrap().clone().unwrap()
}

async fn set_client_token(client: &AuthCodeSpotify, token: Token) {
  *client.token.lock().await.unwrap() = Some(token.clone());
}

impl SpotifyClient {
  pub fn new(settings: &SpotifySettings, kv: Arc<KeyValueStore>) -> Self {
    Self {
      settings: settings.clone(),
      spotify_credential_repository: SpotifyCredentialRepository::new(kv),
    }
  }

  fn base_client(&self) -> AuthCodeSpotify {
    AuthCodeSpotify::new(
      Credentials {
        id: self.settings.client_id.clone(),
        secret: Some(self.settings.client_secret.clone()),
      },
      OAuth {
        redirect_uri: self.settings.redirect_uri.clone(),
        scopes: SCOPES.clone(),
        ..OAuth::default()
      },
    )
  }

  pub async fn is_authorized(&self) -> bool {
    match self.spotify_credential_repository.get().await {
      Ok(Some(_)) => true,
      _ => false,
    }
  }

  pub fn get_authorize_url(&self) -> Result<String> {
    self
      .base_client()
      .get_authorize_url(false)
      .map_err(Into::into)
  }

  pub async fn receive_auth_code(&self, code: &str) -> Result<SpotifyCredentials> {
    let client = self.base_client();
    client.request_token(code).await?;
    let token = get_client_token(&client).await;
    let credentials: SpotifyCredentials = token.into();
    self
      .spotify_credential_repository
      .put(&credentials.clone())
      .await?;

    Ok(credentials)
  }

  async fn client(&self) -> Result<AuthCodeSpotify> {
    let credentials = self
      .spotify_credential_repository
      .get()
      .await?
      .ok_or(anyhow::anyhow!("Credentials not found"))?;
    let client = self.base_client();
    set_client_token(&client, credentials.clone().into()).await;

    if credentials.is_expired() {
      client.refresh_token().await?;
      self
        .spotify_credential_repository
        .put(&get_client_token(&client).await.into())
        .await?;
    }

    Ok(client)
  }

  pub async fn get_saved_tracks(&self) -> Result<Vec<SpotifyTrack>> {
    let client = self.client().await?;
    let (tx, mut rx) = unbounded_channel();
    let stream = client.current_user_saved_tracks(None);
    stream
      .try_for_each_concurrent(1000, |item| {
        let tx = tx.clone();
        async move {
          tx.send(item).unwrap();
          Ok(())
        }
      })
      .await?;
    drop(tx);
    let mut tracks = vec![];
    while let Some(track) = rx.recv().await {
      tracks.push(track.try_into()?);
    }
    Ok(tracks)
  }

  pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<SpotifyTrack>> {
    let client = self.client().await?;
    let (tx, mut rx) = unbounded_channel();
    let stream = client.playlist_items(PlaylistId::from_id(playlist_id)?, None, None);
    stream
      .try_for_each_concurrent(1000, |item| {
        let tx = tx.clone();
        async move {
          tx.send(item).unwrap();
          Ok(())
        }
      })
      .await?;
    drop(tx);
    let mut tracks = vec![];
    while let Some(playlist_item) = rx.recv().await {
      if let Some(PlayableItem::Track(track)) = playlist_item.track {
        match track.try_into() {
          Ok(track) => tracks.push(track),
          Err(err) => {
            warn!("Failed to convert track: {}", err);
          }
        }
      }
    }
    Ok(tracks)
  }

  async fn wait_for_rate_limit(&self) {
    RATE_LIMITER
      .until_ready_with_jitter(Jitter::up_to(std::time::Duration::from_secs(1)))
      .await;
  }

  async fn search(&self, query: String) -> Result<SearchResult> {
    self.wait_for_rate_limit().await;
    let client = self.client().await?;
    let result = client
      .search(query.as_str(), SearchType::Album, None, None, Some(5), None)
      .await
      .map_err(map_spotify_error)?;
    Ok(result)
  }

  async fn album_track(&self, album_id: AlbumId<'static>) -> Result<Vec<SimplifiedTrack>> {
    self.wait_for_rate_limit().await;
    let client = self.client().await?;
    let result = client
      .album_track(album_id, None)
      .try_collect::<Vec<SimplifiedTrack>>()
      .await
      .map_err(map_spotify_error)?;
    Ok(result)
  }

  async fn tracks_features(
    &self,
    track_ids: Vec<TrackId<'static>>,
  ) -> Result<Option<Vec<AudioFeatures>>> {
    let client = self.client().await?;
    let result = client
      .tracks_features(track_ids)
      .await
      .map_err(map_spotify_error)?;
    Ok(result)
  }

  pub async fn find_album(&self, album: &AlbumReadModel) -> Result<Option<SpotifyAlbum>> {
    let query = format!(
      "{} {}",
      album
        .artists
        .first()
        .map(|a| a.name.clone())
        .unwrap_or("".to_string()),
      album.name.clone()
    );
    match self.search(query.clone()).await? {
      SearchResult::Albums(page) => {
        info!(
          file_name = album.file_name.to_string(),
          query,
          results = format!("{:?}", page),
          "Spotify album search results"
        );
        let mut candidates = vec![];
        for item in page.items.into_iter() {
          let name_similarity = jaro_winkler(
            &unidecode(&item.name).to_ascii_lowercase(),
            &album.ascii_name().to_ascii_lowercase(),
          );
          if name_similarity < 0.8 {
            debug!(
              "Album name similarity({}) is too low: {} vs {}",
              name_similarity, item.name, album.name
            );
            continue;
          }
          if item
            .album_type
            .clone()
            .is_some_and(|t| t.eq_ignore_ascii_case("single"))
          {
            debug!("Skipping single album: {}", item.name);
            continue;
          }
          candidates.push((item, name_similarity));
        }

        let candidate_count = candidates.len();
        let match_album = candidates
          .into_iter()
          .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
          .map(|(item, _)| item);

        if let Some(simplified_album) = match_album {
          info!(
            file_name = album.file_name.to_string(),
            name = album.name.clone(),
            spotify_name = simplified_album.name.clone(),
            candidate_count,
            "Found matching album"
          );
          let tracks = self
            .album_track(simplified_album.id.clone().unwrap())
            .await?;

          Ok(Some(get_spotify_album(simplified_album, tracks)))
        } else {
          warn!(
            file_name = album.file_name.to_string(),
            name = album.name.clone(),
            "No matching album found"
          );
          Ok(None)
        }
      }
      _ => Ok(None),
    }
  }

  pub async fn get_tracks_feature_embeddings(
    &self,
    track_uris: Vec<String>,
  ) -> Result<HashMap<String, Vec<f32>>> {
    let mut features = HashMap::new();
    if let Some(results) = self
      .tracks_features(
        track_uris
          .into_iter()
          .filter_map(|id| TrackId::from_id(id.replace("spotify:track:", "")).ok())
          .collect::<Vec<_>>(),
      )
      .await?
    {
      features = results.into_iter().fold(HashMap::new(), |mut acc, f| {
        acc.insert(f.id.to_string(), get_features_embedding(f));
        acc
      });
    }
    Ok(features)
  }

  pub async fn get_track_feature_embeddings(&self, id: String) -> Result<Vec<f32>> {
    let track_id = TrackId::from_id(id.replace("spotify:track:", ""))?;
    let results = self.client().await?.track_features(track_id).await?;
    Ok(get_features_embedding(results))
  }

  pub async fn create_playlist(
    &self,
    name: String,
    description: Option<String>,
    track_uris: Vec<String>,
  ) -> Result<String> {
    let client = self.client().await?;
    let current_user = client.current_user().await?;
    let playlist = client
      .user_playlist_create(
        current_user.id,
        name.as_str(),
        Some(true),
        None,
        description.as_deref(),
      )
      .await?;
    client
      .playlist_add_items(
        playlist.id.clone(),
        track_uris
          .iter()
          .filter_map(|uri| TrackId::from_uri(uri).ok().map(|id| PlayableId::Track(id)))
          .collect::<Vec<_>>(),
        None,
      )
      .await?;
    Ok(playlist.id.to_string())
  }
}
