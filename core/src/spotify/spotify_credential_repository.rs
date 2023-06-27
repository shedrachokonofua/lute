use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use r2d2::Pool;
use redis::{Client, Commands};
use rspotify::scopes;
use std::{collections::HashSet, sync::Arc};

pub struct SpotifyCredentialRepository {
  pub redis_connection_pool: Arc<Pool<Client>>,
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
  fn access_token_key(&self) -> String {
    "spotify:access_token".to_string()
  }

  fn refresh_token_key(&self) -> String {
    "spotify:refresh_token".to_string()
  }

  fn expires_at_key(&self) -> String {
    "spotify:expires_at".to_string()
  }

  pub fn put(&self, credentials: &SpotifyCredentials) -> Result<()> {
    let mut connection = self.redis_connection_pool.get()?;
    let mut transaction = redis::pipe();
    transaction.set(self.access_token_key(), &credentials.access_token);
    transaction.set(self.refresh_token_key(), &credentials.refresh_token);
    transaction.set(self.expires_at_key(), &credentials.expires_at.to_string());
    transaction.query(&mut connection)?;
    Ok(())
  }

  pub fn get_refresh_token(&self) -> Result<Option<String>> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: Option<String> = connection.get(self.refresh_token_key())?;
    Ok(result)
  }

  pub fn get_access_token(&self) -> Result<Option<String>> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: Option<String> = connection.get(self.access_token_key())?;
    Ok(result)
  }

  pub fn get_expires_at(&self) -> Result<Option<NaiveDateTime>> {
    let mut connection = self.redis_connection_pool.get()?;
    let result: Option<String> = connection.get(self.expires_at_key())?;
    match result {
      Some(date) => Ok(Some(NaiveDateTime::parse_from_str(
        &date,
        "%Y-%m-%d %H:%M:%S%.f",
      )?)),
      None => Ok(None),
    }
  }

  pub fn get_credentials(&self) -> Result<Option<SpotifyCredentials>> {
    let access_token = self.get_access_token()?;
    let refresh_token = self.get_refresh_token()?;
    let expires_at = self.get_expires_at()?;
    match (access_token, refresh_token, expires_at) {
      (Some(access_token), Some(refresh_token), Some(expires_at)) => Ok(Some(SpotifyCredentials {
        access_token,
        refresh_token,
        expires_at,
      })),
      _ => Ok(None),
    }
  }

  pub fn delete(&self) -> Result<()> {
    let mut connection = self.redis_connection_pool.get()?;
    let mut transaction = redis::pipe();
    transaction.del(self.access_token_key());
    transaction.del(self.refresh_token_key());
    transaction.del(self.expires_at_key());
    transaction.query(&mut connection)?;
    Ok(())
  }
}
