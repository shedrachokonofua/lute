use super::artist_read_model::{ArtistReadModel, ArtistReadModelCredit};
use crate::{files::file_metadata::file_name::FileName, sqlite::SqliteConnection};
use anyhow::{anyhow, Result};
use rusqlite::types::Value;
use std::{collections::HashMap, rc::Rc, sync::Arc};
use tokio::try_join;
use tracing::{error, instrument};

pub struct ArtistRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

impl ArtistRepository {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(skip(self))]
  async fn find_album_file_names(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, Vec<FileName>>> {
    let artist_file_name_params = artist_file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();

    let rows = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            albums.file_name,
            artists.file_name
          FROM albums
          JOIN album_artists ON albums.id = album_artists.album_id
          JOIN artists ON album_artists.artist_id = artists.id
          WHERE artists.file_name IN rarray(?)
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(artist_file_name_params)], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
          })?
          .collect::<Result<Vec<(String, String)>, _>>();
        rows.inspect_err(|e| {
          error!(message = e.to_string(), "Failed to find album entities");
        })
      })
      .await
      .map_err(|e| anyhow!("Failed to find album entities {:?}", e))??;

    let mut album_file_names = HashMap::new();
    for (album_file_name, artist_file_name) in rows {
      let artist_file_name = FileName::try_from(artist_file_name)?;
      let album_file_name = FileName::try_from(album_file_name)?;
      album_file_names
        .entry(artist_file_name)
        .or_insert_with(Vec::new)
        .push(album_file_name);
    }

    Ok(album_file_names)
  }

  #[instrument(skip(self))]
  async fn find_credits(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, Vec<ArtistReadModelCredit>>> {
    let artist_file_name_params = artist_file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();

    let rows = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            artists.file_name,
            albums.file_name,
            roles.name
          FROM credits
          JOIN artists ON credits.artist_id = artists.id
          JOIN albums ON credits.album_id = albums.id
          JOIN credit_roles ON credits.id = credit_roles.credit_id
          JOIN roles ON credit_roles.role_id = roles.id
          JOIN album_artists ON albums.id = album_artists.album_id
          WHERE artists.file_name IN rarray(?)
          AND credits.album_id NOT IN (
            SELECT album_id
            FROM album_artists
            WHERE artist_id = artists.id
          )
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(artist_file_name_params)], |row| {
            Ok((
              row.get::<_, String>(0)?,
              row.get::<_, String>(1)?,
              row.get::<_, String>(2)?,
            ))
          })?
          .collect::<Result<Vec<_>, _>>();
        rows.inspect_err(|e| {
          error!(message = e.to_string(), "Failed to find credits");
        })
      })
      .await
      .map_err(|e| anyhow!("Failed to find credits: {:?}", e))??;

    let mut credits: HashMap<FileName, HashMap<FileName, Vec<String>>> = HashMap::new();
    for (artist_file_name, album_file_name, role) in rows {
      let artist_file_name = FileName::try_from(artist_file_name)?;
      let album_file_name = FileName::try_from(album_file_name)?;

      credits
        .entry(artist_file_name)
        .or_default()
        .entry(album_file_name)
        .or_default()
        .push(role);
    }

    let credits: HashMap<FileName, Vec<ArtistReadModelCredit>> = credits
      .into_iter()
      .map(|(artist_file_name, album_roles)| {
        let artist_file_name = artist_file_name.clone();
        let album_roles = album_roles
          .into_iter()
          .map(|(album_file_name, roles)| ArtistReadModelCredit {
            album_file_name,
            roles,
          })
          .collect();
        (artist_file_name, album_roles)
      })
      .collect();

    Ok(credits)
  }

  #[instrument(skip(self))]
  pub async fn find_many(
    &self,
    artist_file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, ArtistReadModel>> {
    let (album_file_names, credits) = try_join!(
      self.find_album_file_names(artist_file_names.clone()),
      self.find_credits(artist_file_names.clone())
    )?;

    let artist_file_name_params = artist_file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();

    let rows = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn
          .prepare("SELECT file_name, name FROM artists WHERE artists.file_name IN rarray(?)")?;
        let rows = stmt
          .query_map([Rc::new(artist_file_name_params)], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
          })?
          .collect::<Result<Vec<_>, _>>();
        rows.inspect_err(|e| {
          error!(message = e.to_string(), "Failed to find credits");
        })
      })
      .await
      .map_err(|e| anyhow!("Failed to find credits {:?}", e))??;

    let mut artists = HashMap::new();
    for (file_name, name) in rows {
      let file_name = FileName::try_from(file_name)?;
      let album_file_names = album_file_names
        .get(&file_name)
        .cloned()
        .unwrap_or_default();
      let credits = credits.get(&file_name).cloned().unwrap_or_default();
      artists.insert(
        file_name.clone(),
        ArtistReadModel {
          name,
          file_name,
          album_file_names,
          credits,
        },
      );
    }

    Ok(artists)
  }
}
