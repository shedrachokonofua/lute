use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use rspotify::scopes;
use rustis::{
  bb8::Pool,
  client::{BatchPreparedCommand, PooledClientManager},
  commands::GenericCommands,
  commands::StringCommands,
};
use std::{collections::HashSet, sync::Arc};

pub struct SpotifyCredentialRepository {
  pub redis_connection_pool: Arc<Pool<PooledClientManager>>,
}

#[derive(Debug, Clone)]
pub struct SpotifyCredentials {
  pub access_token: String,
  pub refresh_token: String,
  pub expires_at: NaiveDateTime,
}

impl SpotifyCredentials {
  pub fn scopes() -> HashSet<String> {
    scopes!("user-library-read", "user-top-read")
  }

  pub fn is_expired(&self) -> bool {
    self.expires_at < Utc::now().naive_utc()
  }
}

impl SpotifyCredentialRepository {
  fn access_token_key(&self) -> &str {
    "spotify:access_token"
  }

  fn refresh_token_key(&self) -> &str {
    "spotify:refresh_token"
  }

  fn expires_at_key(&self) -> &str {
    "spotify:expires_at"
  }

  pub async fn put(&self, credentials: &SpotifyCredentials) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    let mut transaction = connection.create_transaction();
    transaction
      .set(self.access_token_key(), &credentials.access_token)
      .forget();
    transaction
      .set(self.refresh_token_key(), &credentials.refresh_token)
      .forget();
    transaction
      .set(self.expires_at_key(), &credentials.expires_at.to_string())
      .queue();
    transaction.execute().await?;
    Ok(())
  }

  pub async fn get_refresh_token(&self) -> Result<Option<String>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection.get(self.refresh_token_key()).await?;
    Ok(result)
  }

  pub async fn get_access_token(&self) -> Result<Option<String>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection.get(self.access_token_key()).await?;
    Ok(result)
  }

  pub async fn get_expires_at(&self) -> Result<Option<NaiveDateTime>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection.get(self.expires_at_key()).await?;
    match result {
      Some(date) => Ok(Some(NaiveDateTime::parse_from_str(
        &date,
        "%Y-%m-%d %H:%M:%S%.f",
      )?)),
      None => Ok(None),
    }
  }

  pub async fn get_credentials(&self) -> Result<Option<SpotifyCredentials>> {
    let access_token = self.get_access_token().await?;
    let refresh_token = self.get_refresh_token().await?;
    let expires_at = self.get_expires_at().await?;
    match (access_token, refresh_token, expires_at) {
      (Some(access_token), Some(refresh_token), Some(expires_at)) => Ok(Some(SpotifyCredentials {
        access_token,
        refresh_token,
        expires_at,
      })),
      _ => Ok(None),
    }
  }

  pub async fn delete(&self) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    connection
      .del(vec![
        self.access_token_key(),
        self.refresh_token_key(),
        self.expires_at_key(),
      ])
      .await?;
    Ok(())
  }
}
