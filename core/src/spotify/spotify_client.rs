use super::spotify_credential_repository::{
  SpotifyCredentialRepository, SpotifyCredentials, SCOPES,
};
use crate::settings::SpotifySettings;
use anyhow::{anyhow, Error, Result};
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use rspotify::{
  model::{FullTrack, PlayableItem, PlaylistId, SavedTrack, SimplifiedAlbum, SimplifiedArtist},
  prelude::{BaseClient, OAuthClient},
  AuthCodeSpotify, Credentials, OAuth, Token,
};
use rustis::bb8::Pool;
use rustis::client::PooledClientManager;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use tracing::warn;

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

pub struct SpotifyArtistReference {
  pub spotify_id: String,
  pub name: String,
}

#[derive(PartialEq)]
pub enum SpotifyAlbumType {
  Album,
  Single,
  Compilation,
}

pub struct SpotifyAlbumReference {
  pub spotify_id: String,
  pub name: String,
  pub album_type: SpotifyAlbumType,
}

pub struct SpotifyTrack {
  pub spotify_id: String,
  pub name: String,
  pub artists: Vec<SpotifyArtistReference>,
  pub album: SpotifyAlbumReference,
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
  pub fn new(
    settings: &SpotifySettings,
    redis_connection_pool: Arc<Pool<PooledClientManager>>,
  ) -> Self {
    Self {
      settings: settings.clone(),
      spotify_credential_repository: SpotifyCredentialRepository {
        redis_connection_pool,
      },
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
    let creds = self.spotify_credential_repository.get_credentials().await;
    creds.is_ok() && creds.unwrap().is_some()
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
      .get_credentials()
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
}
