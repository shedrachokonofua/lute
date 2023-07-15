use super::spotify_credential_repository::{SpotifyCredentialRepository, SpotifyCredentials};
use crate::settings::SpotifySettings;
use anyhow::Result;
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
    let expires_at: DateTime<Utc> = DateTime::from_utc(credentials.expires_at, Utc);

    Self {
      scopes: SpotifyCredentials::scopes(),
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

impl From<SimplifiedAlbum> for SpotifyAlbumReference {
  fn from(simplified_album: SimplifiedAlbum) -> Self {
    SpotifyAlbumReference {
      spotify_id: simplified_album.id.unwrap().to_string(),
      name: simplified_album.name,
      album_type: match simplified_album.album_type.unwrap().as_str() {
        "album" => SpotifyAlbumType::Album,
        "single" => SpotifyAlbumType::Single,
        "compilation" => SpotifyAlbumType::Compilation,
        _ => panic!("Unknown album type"),
      },
    }
  }
}

impl From<&SimplifiedArtist> for SpotifyArtistReference {
  fn from(simplified_artist: &SimplifiedArtist) -> Self {
    SpotifyArtistReference {
      spotify_id: simplified_artist.id.clone().unwrap().to_string(),
      name: simplified_artist.name.clone(),
    }
  }
}

impl From<SavedTrack> for SpotifyTrack {
  fn from(saved_track: SavedTrack) -> Self {
    SpotifyTrack {
      spotify_id: saved_track.track.id.unwrap().to_string(),
      name: saved_track.track.name,
      artists: saved_track
        .track
        .artists
        .iter()
        .map(|artist| artist.into())
        .collect(),
      album: saved_track.track.album.into(),
    }
  }
}

impl From<FullTrack> for SpotifyTrack {
  fn from(full_track: FullTrack) -> Self {
    SpotifyTrack {
      spotify_id: full_track.id.unwrap().to_string(),
      name: full_track.name,
      artists: full_track
        .artists
        .iter()
        .map(|artist| artist.into())
        .collect(),
      album: full_track.album.into(),
    }
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
        scopes: SpotifyCredentials::scopes(),
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
      tracks.push(track.into());
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
      if let Some(playable_item) = playlist_item.track {
        match playable_item {
          PlayableItem::Track(track) => tracks.push(track.into()),
          _ => {}
        }
      }
    }
    Ok(tracks)
  }
}
