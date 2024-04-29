use crate::helpers::key_value_store::KeyValueStore;
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use lazy_static::lazy_static;
use rspotify::scopes;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub struct SpotifyCredentialRepository {
  kv: Arc<KeyValueStore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyCredentials {
  pub access_token: String,
  pub refresh_token: String,
  pub expires_at: NaiveDateTime,
}

lazy_static! {
  pub static ref SCOPES: HashSet<String> = scopes!("user-library-read", "user-top-read");
}

const KEY: &str = "spotify:credentials";

impl SpotifyCredentials {
  pub fn is_expired(&self) -> bool {
    self.expires_at < Utc::now().naive_utc()
  }
}

impl SpotifyCredentialRepository {
  pub fn new(kv: Arc<KeyValueStore>) -> Self {
    Self { kv }
  }

  pub async fn put(&self, credentials: &SpotifyCredentials) -> Result<()> {
    self.kv.set(KEY, credentials, None).await
  }

  pub async fn get(&self) -> Result<Option<SpotifyCredentials>> {
    self.kv.get::<SpotifyCredentials>(KEY).await
  }

  pub async fn delete(&self) -> Result<()> {
    self.kv.delete(KEY).await
  }
}
