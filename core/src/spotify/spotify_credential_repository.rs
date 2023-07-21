use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use lazy_static::lazy_static;
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

const ACCESS_TOKEN_KEY: &str = "spotify:access_token";
const REFRESH_TOKEN_KEY: &str = "spotify:refresh_token";
const EXPIRES_AT_KEY: &str = "spotify:expires_at";
lazy_static! {
  pub static ref SCOPES: HashSet<String> = scopes!("user-library-read", "user-top-read");
}

impl SpotifyCredentials {
  pub fn is_expired(&self) -> bool {
    self.expires_at < Utc::now().naive_utc()
  }
}

impl SpotifyCredentialRepository {
  pub async fn put(&self, credentials: &SpotifyCredentials) -> Result<()> {
    let connection = self.redis_connection_pool.get().await?;
    let mut transaction = connection.create_transaction();
    transaction
      .set(ACCESS_TOKEN_KEY, &credentials.access_token)
      .forget();
    transaction
      .set(REFRESH_TOKEN_KEY, &credentials.refresh_token)
      .forget();
    transaction
      .set(EXPIRES_AT_KEY, &credentials.expires_at.to_string())
      .queue();
    transaction.execute().await?;
    Ok(())
  }

  pub async fn get_refresh_token(&self) -> Result<Option<String>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection.get(REFRESH_TOKEN_KEY).await?;
    Ok(result)
  }

  pub async fn get_access_token(&self) -> Result<Option<String>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection.get(ACCESS_TOKEN_KEY).await?;
    Ok(result)
  }

  pub async fn get_expires_at(&self) -> Result<Option<NaiveDateTime>> {
    let connection = self.redis_connection_pool.get().await?;
    let result: Option<String> = connection.get(EXPIRES_AT_KEY).await?;
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
      .del(vec![ACCESS_TOKEN_KEY, REFRESH_TOKEN_KEY, EXPIRES_AT_KEY])
      .await?;
    Ok(())
  }
}
