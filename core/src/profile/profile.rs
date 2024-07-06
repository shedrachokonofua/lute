use crate::files::file_metadata::file_name::FileName;
use anyhow::{bail, Result};
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

lazy_static! {
  static ref PROFILE_ID_RE: Regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{2,80}$").unwrap();
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct ProfileId(String);

impl TryFrom<String> for ProfileId {
  type Error = anyhow::Error;

  fn try_from(value: String) -> Result<Self> {
    if PROFILE_ID_RE.is_match(&value) {
      Ok(Self(value))
    } else {
      bail!("Invalid profile name: {}", value)
    }
  }
}

impl ToString for ProfileId {
  fn to_string(&self) -> String {
    self.0.clone()
  }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Profile {
  pub id: ProfileId,
  pub name: String,
  pub albums: HashMap<FileName, u32>,
  pub last_updated_at: NaiveDateTime,
}

impl Profile {
  pub fn album_file_names(&self) -> Vec<FileName> {
    self.albums.keys().cloned().collect()
  }
}
