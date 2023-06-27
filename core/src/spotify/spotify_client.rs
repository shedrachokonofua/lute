use super::spotify_credential_repository::SpotifyCredentialRepository;
use super::spotify_credential_repository::SpotifyCredentials;
use crate::settings::SpotifySettings;
use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use r2d2::Pool;
use redis::Client;
use rspotify::prelude::BaseClient;
use rspotify::{prelude::OAuthClient, AuthCodeSpotify, Credentials, OAuth, Token};
use std::sync::Arc;

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
  pub fn new(settings: &SpotifySettings, redis_connection_pool: Arc<Pool<Client>>) -> Self {
    Self {
      settings: settings.clone(),
      spotify_credential_repository: SpotifyCredentialRepository {
        redis_connection_pool: redis_connection_pool.clone(),
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
    let creds = self.spotify_credential_repository.get_credentials();
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
      .put(&credentials.clone())?;

    Ok(credentials)
  }

  async fn client(&self) -> Result<AuthCodeSpotify> {
    let credentials = self
      .spotify_credential_repository
      .get_credentials()?
      .ok_or(anyhow::anyhow!("Credentials not found"))?;
    let client = self.base_client();
    set_client_token(&client, credentials.clone().into()).await;

    if credentials.is_expired() {
      client.refresh_token().await?;
      self
        .spotify_credential_repository
        .put(&get_client_token(&client).await.into())?;
    }

    Ok(client)
  }
}
