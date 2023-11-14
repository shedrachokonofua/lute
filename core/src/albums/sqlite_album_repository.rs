use super::{
  album_read_model::{
    AlbumReadModel, AlbumReadModelArtist, AlbumReadModelCredit, AlbumReadModelTrack,
  },
  album_repository::{AlbumRepository, GenreAggregate, ItemAndCount},
};
use crate::{files::file_metadata::file_name::FileName, sqlite::SqliteConnection};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use rusqlite::{params, types::Value};
use std::{
  collections::{HashMap, HashSet},
  rc::Rc,
  sync::Arc,
};
use tokio::try_join;
use tracing::{error, instrument};

pub struct SqliteAlbumRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

enum AlbumDuplication {
  Duplicates(Vec<FileName>),
  DuplicateOf(FileName),
}

struct AlbumEntity {
  pub id: i64,
  pub name: String,
  pub file_name: FileName,
  pub rating: f32,
  pub rating_count: u32,
  pub release_date: Option<NaiveDate>,
  pub cover_image_url: Option<String>,
}

impl SqliteAlbumRepository {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(skip_all, fields(count = file_names.len()))]
  async fn find_album_entities(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<HashMap<FileName, AlbumEntity>> {
    let file_name_params = file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            id,
            file_name,
            name,
            rating,
            rating_count,
            release_date,
            cover_image_url
          FROM albums
          WHERE file_name IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(file_name_params)], |row| {
          Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, f32>(3)?,
            row.get::<_, u32>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
          ))
        })?;
        let mut result = HashMap::<FileName, AlbumEntity>::new();
        while let Some(Ok(row)) = rows.next() {
          let (id, file_name, name, rating, rating_count, release_date, cover_image_url) = row;
          let file_name = FileName::try_from(file_name.clone()).map_err(|e| {
            error!(message = e.to_string(), "Failed to parse album file name");
            rusqlite::Error::ExecuteReturnedResults
          })?;
          result.insert(
            file_name.clone(),
            AlbumEntity {
              id,
              name,
              file_name,
              rating,
              rating_count,
              release_date: release_date
                .map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").unwrap()),
              cover_image_url,
            },
          );
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album entities");
        anyhow!("Failed to find album entities")
      })?
  }

  #[instrument(skip_all, fields(count = album_ids.len()))]
  async fn find_album_artists(
    &self,
    album_ids: Vec<i64>,
  ) -> Result<HashMap<i64, Vec<AlbumReadModelArtist>>> {
    let album_id_params = album_ids
      .into_iter()
      .map(|f| Value::from(f))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            album_artists.album_id,
            artists.file_name,
            artists.name
          FROM album_artists
          LEFT JOIN artists ON album_artists.artist_id = artists.id
          WHERE album_artists.album_id IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(album_id_params)], |row| {
          Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
          ))
        })?;
        let mut result = HashMap::<i64, Vec<AlbumReadModelArtist>>::new();
        while let Some(Ok(row)) = rows.next() {
          let (album_id, artist_file_name, artist_name) = row;
          let album_entry = result.entry(album_id).or_insert_with(|| Vec::new());
          album_entry.push(AlbumReadModelArtist {
            file_name: FileName::try_from(artist_file_name.clone()).map_err(|e| {
              error!(message = e.to_string(), "Failed to parse artist file name");
              rusqlite::Error::ExecuteReturnedResults
            })?,
            name: artist_name,
          });
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album artists");
        anyhow!("Failed to find album artists")
      })?
  }

  #[instrument(skip_all, fields(count = album_ids.len()))]
  async fn find_album_genres(
    &self,
    album_ids: Vec<i64>,
  ) -> Result<HashMap<i64, (Vec<String>, Vec<String>)>> {
    let album_id_params = album_ids
      .into_iter()
      .map(|f| Value::from(f))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            album_genres.album_id,
            genres.name,
            album_genres.is_primary
          FROM album_genres
          LEFT JOIN genres ON album_genres.genre_id = genres.id
          WHERE album_genres.album_id IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(album_id_params)], |row| {
          Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
          ))
        })?;
        let mut result = HashMap::<i64, (Vec<String>, Vec<String>)>::new();
        while let Some(Ok(row)) = rows.next() {
          let (album_id, genre_name, is_primary) = row;
          let album_entry = result
            .entry(album_id)
            .or_insert_with(|| (Vec::new(), Vec::new()));
          if is_primary {
            album_entry.0.push(genre_name);
          } else {
            album_entry.1.push(genre_name);
          }
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album genres");
        anyhow!("Failed to find album genres")
      })?
  }

  #[instrument(skip_all, fields(count = album_ids.len()))]
  async fn find_album_descriptors(&self, album_ids: Vec<i64>) -> Result<HashMap<i64, Vec<String>>> {
    let album_id_params = album_ids
      .into_iter()
      .map(|f| Value::from(f))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            album_descriptors.album_id,
            descriptors.name
          FROM album_descriptors
          LEFT JOIN descriptors ON album_descriptors.descriptor_id = descriptors.id
          WHERE album_descriptors.album_id IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(album_id_params)], |row| {
          Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut result = HashMap::<i64, Vec<String>>::new();
        while let Some(Ok(row)) = rows.next() {
          let (album_id, descriptor_name) = row;
          let album_entry = result.entry(album_id).or_insert_with(|| Vec::new());
          album_entry.push(descriptor_name);
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album descriptors");
        anyhow!("Failed to find album descriptors")
      })?
  }

  #[instrument(skip_all, fields(count = album_ids.len()))]
  async fn find_album_languages(&self, album_ids: Vec<i64>) -> Result<HashMap<i64, Vec<String>>> {
    let album_id_params = album_ids
      .into_iter()
      .map(|f| Value::from(f))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            album_languages.album_id,
            languages.name
          FROM album_languages
          LEFT JOIN languages ON album_languages.language_id = languages.id
          WHERE album_languages.album_id IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(album_id_params)], |row| {
          Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut result = HashMap::<i64, Vec<String>>::new();
        while let Some(Ok(row)) = rows.next() {
          let (album_id, language_name) = row;
          let album_entry = result.entry(album_id).or_insert_with(|| Vec::new());
          album_entry.push(language_name);
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album languages");
        anyhow!("Failed to find album languages")
      })?
  }

  #[instrument(skip_all, fields(count = album_ids.len()))]
  async fn find_album_tracks(
    &self,
    album_ids: Vec<i64>,
  ) -> Result<HashMap<i64, Vec<AlbumReadModelTrack>>> {
    let album_id_params = album_ids
      .into_iter()
      .map(|f| Value::from(f))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            tracks.album_id,
            tracks.name,
            tracks.duration_seconds,
            tracks.rating,
            tracks.position
          FROM tracks
          WHERE tracks.album_id IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(album_id_params)], |row| {
          Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<u32>>(2)?,
            row.get::<_, Option<f32>>(3)?,
            row.get::<_, Option<String>>(4)?,
          ))
        })?;
        let mut result = HashMap::<i64, Vec<AlbumReadModelTrack>>::new();
        while let Some(Ok(row)) = rows.next() {
          let (album_id, track_name, track_duration_seconds, track_rating, track_position) = row;
          let album_entry = result.entry(album_id).or_insert_with(|| Vec::new());
          album_entry.push(AlbumReadModelTrack {
            name: track_name,
            duration_seconds: track_duration_seconds,
            rating: track_rating,
            position: track_position,
          });
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album tracks");
        anyhow!("Failed to find album tracks")
      })?
  }

  #[instrument(skip_all, fields(count = album_ids.len()))]
  async fn find_album_credits(
    &self,
    album_ids: Vec<i64>,
  ) -> Result<HashMap<i64, Vec<AlbumReadModelCredit>>> {
    let album_id_params = album_ids
      .into_iter()
      .map(|f| Value::from(f))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            credits.album_id,
            artists.file_name,
            artists.name,
            roles.name
          FROM credits
          LEFT JOIN artists ON credits.artist_id = artists.id
          LEFT JOIN credit_roles ON credits.id = credit_roles.credit_id
          LEFT JOIN roles ON credit_roles.role_id = roles.id
          WHERE credits.album_id IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(album_id_params)], |row| {
          Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
          ))
        })?;
        let mut result = HashMap::<i64, Vec<AlbumReadModelCredit>>::new();
        while let Some(Ok(row)) = rows.next() {
          let (album_id, artist_file_name, artist_name, role) = row;
          let album_entry = result.entry(album_id).or_insert_with(|| Vec::new());
          let artist_file_name = FileName::try_from(artist_file_name.clone()).map_err(|e| {
            error!(message = e.to_string(), "Failed to parse artist file name");
            rusqlite::Error::ExecuteReturnedResults
          })?;
          let credit_entry = album_entry
            .iter_mut()
            .find(|credit| credit.artist.file_name == artist_file_name);
          match credit_entry {
            Some(credit_entry) => {
              credit_entry.roles.push(role);
            }
            None => {
              album_entry.push(AlbumReadModelCredit {
                artist: AlbumReadModelArtist {
                  file_name: artist_file_name,
                  name: artist_name,
                },
                roles: vec![role],
              });
            }
          }
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album credits");
        anyhow!("Failed to find album credits")
      })?
  }

  #[instrument(skip_all, fields(count = album_ids.len()))]
  async fn find_album_duplication(
    &self,
    album_ids: Vec<i64>,
  ) -> Result<HashMap<i64, AlbumDuplication>> {
    let album_id_params = album_ids
      .into_iter()
      .map(|f| Value::from(f))
      .collect::<Vec<Value>>();

    self
      .sqlite_connection
      .read().await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT
            album_duplicates.original_album_id,
            album_duplicates.duplicate_album_id,
            original_albums.file_name,
            duplicate_albums.file_name
          FROM album_duplicates
          LEFT JOIN albums original_albums ON album_duplicates.original_album_id = original_albums.id
          LEFT JOIN albums duplicate_albums ON album_duplicates.duplicate_album_id = duplicate_albums.id
          WHERE album_duplicates.original_album_id IN rarray(?1) OR album_duplicates.duplicate_album_id IN rarray(?1)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(album_id_params)], |row| {
          Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
          ))
        })?;
        let mut result = HashMap::<i64, AlbumDuplication>::new();
        while let Some(Ok((
          original_album_id,
          duplicate_album_id,
          original_album_file_name,
          duplicate_album_file_name,
        ))) = rows.next() {
          let original_album_file_name = FileName::try_from(original_album_file_name.clone())
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to parse album file name");
              rusqlite::Error::ExecuteReturnedResults
            })?;
          let original_album_entry = result
            .entry(original_album_id)
            .or_insert_with(|| AlbumDuplication::Duplicates(Vec::new()));
          let duplicate_album_file_name = FileName::try_from(duplicate_album_file_name.clone())
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to parse album file name");
              rusqlite::Error::ExecuteReturnedResults
            })?;
          match original_album_entry {
            AlbumDuplication::Duplicates(duplicates) => {
              duplicates.push(duplicate_album_file_name);
            }
            AlbumDuplication::DuplicateOf(_) => {
              panic!(
                "Album {} is both a duplicate and a duplicate of another album",
                original_album_file_name.to_string()
              );
            }
          }
          result.insert(
            duplicate_album_id,
            AlbumDuplication::DuplicateOf(original_album_file_name.clone()),
          );
        }
        Ok(result)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find album duplicates");
        anyhow!("Failed to find album duplicates")
      })?
  }
}

#[async_trait]
impl AlbumRepository for SqliteAlbumRepository {
  #[instrument(skip_all, fields(file_name = album.file_name.to_string()))]
  async fn put(&self, album: AlbumReadModel) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        tx.execute(
          "
          INSERT INTO albums (file_name, name, rating, rating_count, release_date, cover_image_url)
          VALUES (?, ?, ?, ?, ?, ?)
          ON CONFLICT (file_name) DO UPDATE SET
            name = excluded.name,
            rating = excluded.rating,
            rating_count = excluded.rating_count,
            release_date = excluded.release_date,
            cover_image_url = excluded.cover_image_url
          ",
          params![
            album.file_name.to_string(),
            album.name,
            album.rating,
            album.rating_count,
            album.release_date,
            album.cover_image_url
          ],
        )?;
        let album_id: i64 = tx.query_row(
          "SELECT id FROM albums WHERE file_name = ?",
          params![album.file_name.to_string()],
          |row| row.get(0),
        )?;

        tx.execute(
          "
          DELETE FROM album_artists WHERE album_id = ?
          ",
          params![album_id],
        )?;
        for artist in album.artists {
          let artist_id: i64 = tx.query_row(
            "
            INSERT INTO artists (file_name, name) 
            VALUES (?, ?) 
            ON CONFLICT(file_name) DO UPDATE SET name = excluded.name
            RETURNING id
            ",
            params![artist.file_name.to_string(), artist.name],
            |row| row.get(0),
          )?;
          tx.execute(
            "
            INSERT INTO album_artists (album_id, artist_id)
            VALUES (?, ?)
            ",
            params![album_id, artist_id],
          )?;
        }

        tx.execute(
          "
          DELETE FROM album_genres WHERE album_id = ?
          ",
          params![album_id],
        )?;
        for genre in album.primary_genres {
          let genre_id: i64 = tx.query_row(
            "
            INSERT INTO genres (name) 
            VALUES (?) 
            ON CONFLICT(name) DO UPDATE SET name = excluded.name
            RETURNING id
            ",
            params![genre],
            |row| row.get(0),
          )?;
          tx.execute(
            "
            INSERT INTO album_genres (album_id, genre_id, is_primary)
            VALUES (?, ?, ?)
            ",
            params![album_id, genre_id, true],
          )?;
        }
        for genre in album.secondary_genres {
          let genre_id: i64 = tx.query_row(
            "
            INSERT INTO genres (name) 
            VALUES (?) 
            ON CONFLICT(name) DO UPDATE SET name = excluded.name
            RETURNING id
            ",
            params![genre],
            |row| row.get(0),
          )?;
          tx.execute(
            "
            INSERT INTO album_genres (album_id, genre_id, is_primary)
            VALUES (?, ?, ?)
            ",
            params![album_id, genre_id, false],
          )?;
        }

        tx.execute(
          "
          DELETE FROM album_descriptors WHERE album_id = ?
          ",
          params![album_id],
        )?;
        for descriptor in album.descriptors {
          let descriptor_id: i64 = tx.query_row(
            "
            INSERT INTO descriptors (name) 
            VALUES (?) 
            ON CONFLICT(name) DO UPDATE SET name = excluded.name
            RETURNING id
            ",
            params![descriptor],
            |row| row.get(0),
          )?;
          tx.execute(
            "
            INSERT INTO album_descriptors (album_id, descriptor_id)
            VALUES (?, ?)
            ",
            params![album_id, descriptor_id],
          )?;
        }

        tx.execute(
          "
          DELETE FROM album_languages WHERE album_id = ?
          ",
          params![album_id],
        )?;
        for language in album.languages {
          let language_id: i64 = tx.query_row(
            "
            INSERT INTO languages (name) 
            VALUES (?) 
            ON CONFLICT(name) DO UPDATE SET name = excluded.name
            RETURNING id
            ",
            params![language],
            |row| row.get(0),
          )?;
          tx.execute(
            "
            INSERT INTO album_languages (album_id, language_id)
            VALUES (?, ?)
            ",
            params![album_id, language_id],
          )?;
        }

        tx.execute(
          "
          DELETE FROM tracks WHERE album_id = ?
          ",
          params![album_id],
        )?;
        for track in album.tracks {
          tx.execute(
            "
            INSERT INTO tracks (album_id, name, duration_seconds, rating, position)
            VALUES (?, ?, ?, ?, ?)
            ",
            params![
              album_id,
              track.name,
              track.duration_seconds,
              track.rating,
              track.position,
            ],
          )?;
        }

        tx.execute(
          "
          DELETE FROM album_duplicates 
          WHERE original_album_id = ?1 OR duplicate_album_id = ?1
          ",
          params![album_id],
        )?;
        for duplicate in album.duplicates {
          let duplicate_id: i64 = tx.query_row(
            "SELECT id FROM albums WHERE file_name = ?",
            params![duplicate.to_string()],
            |row| row.get(0),
          )?;
          tx.execute(
            "
            INSERT INTO album_duplicates (original_album_id, duplicate_album_id)
            VALUES (?, ?)
            ",
            params![album_id, duplicate_id],
          )?;
        }
        if let Some(duplicate_of) = album.duplicate_of {
          let duplicate_of_id: i64 = tx.query_row(
            "SELECT id FROM albums WHERE file_name = ?",
            params![duplicate_of.to_string()],
            |row| row.get(0),
          )?;
          tx.execute(
            "
            INSERT INTO album_duplicates (original_album_id, duplicate_album_id)
            VALUES (?, ?)
            ",
            params![duplicate_of_id, album_id],
          )?;
        }

        // credits
        tx.execute(
          "
          DELETE FROM credits WHERE album_id = ?
          ",
          params![album_id],
        )?;
        for credit in album.credits {
          let artist_id: i64 = tx.query_row(
            "
            INSERT INTO artists (file_name, name) 
            VALUES (?, ?) 
            ON CONFLICT(file_name) DO UPDATE SET name = excluded.name
            RETURNING id
            ",
            params![credit.artist.file_name.to_string(), credit.artist.name],
            |row| row.get(0),
          )?;
          let credit_id: i64 = tx.query_row(
            "
            INSERT INTO credits (album_id, artist_id)
            VALUES (?, ?)
            RETURNING id
            ",
            params![album_id, artist_id],
            |row| row.get(0),
          )?;
          for role in credit.roles {
            let role_id: i64 = tx.query_row(
              "
              INSERT INTO roles (name) 
              VALUES (?) 
              ON CONFLICT(name) DO UPDATE SET name = excluded.name
              RETURNING id
              ",
              params![role],
              |row| row.get(0),
            )?;
            tx.execute(
              "
              INSERT INTO credit_roles (credit_id, role_id)
              VALUES (?, ?)
              ",
              params![credit_id, role_id],
            )?;
          }
        }
        tx.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to put album");
        anyhow!("Failed to put album")
      })?
  }

  #[instrument(skip_all, fields(file_name, count = duplicates.len()))]
  async fn set_duplicates(&self, file_name: &FileName, duplicates: Vec<FileName>) -> Result<()> {
    match self.find(file_name).await? {
      Some(_) => {
        let file_name = file_name.to_string();
        self
          .sqlite_connection
          .write()
          .await?
          .interact(move |conn| {
            let tx = conn.transaction()?;
            let album_id: i64 = tx.query_row(
              "SELECT id FROM albums WHERE file_name = ?",
              params![file_name],
              |row| row.get(0),
            )?;
            tx.execute(
              "DELETE FROM album_duplicates WHERE original_album_id = ?1",
              params![album_id],
            )?;
            for duplicate in duplicates {
              let duplicate_id: i64 = tx.query_row(
                "SELECT id FROM albums WHERE file_name = ?",
                params![duplicate.to_string()],
                |row| row.get(0),
              )?;
              tx.execute(
                "
                INSERT INTO album_duplicates (original_album_id, duplicate_album_id)
                VALUES (?, ?)
                ",
                params![album_id, duplicate_id],
              )?;
            }
            tx.commit()?;
            Ok(())
          })
          .await
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to set album duplicates");
            anyhow!("Failed to set album duplicates")
          })?
      }
      None => Err(anyhow!("Album not found")),
    }
  }

  #[instrument(skip(self))]
  async fn set_duplicate_of(&self, file_name: &FileName, duplicate_of: &FileName) -> Result<()> {
    match self.find(file_name).await? {
      Some(_) => {
        let file_name = file_name.to_string();
        let duplicate_of = duplicate_of.to_string();
        self
          .sqlite_connection
          .write()
          .await?
          .interact(move |conn| {
            let tx = conn.transaction()?;
            let album_id: i64 = tx.query_row(
              "SELECT id FROM albums WHERE file_name = ?",
              params![file_name],
              |row| row.get(0),
            )?;
            let duplicate_of_id: i64 = tx.query_row(
              "SELECT id FROM albums WHERE file_name = ?",
              params![duplicate_of],
              |row| row.get(0),
            )?;
            tx.execute(
              "
              INSERT INTO album_duplicates (original_album_id, duplicate_album_id)
              VALUES (?, ?)
              ON CONFLICT (duplicate_album_id) DO UPDATE SET original_album_id = excluded.original_album_id
              ",
              params![duplicate_of_id, album_id],
            )?;
            tx.commit()?;
            Ok(())
          })
          .await
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to set album duplicate of");
            anyhow!("Failed to set album duplicate of")
          })?
      }
      None => Err(anyhow!("Album not found")),
    }
  }

  #[instrument(skip_all, fields(file_name))]
  async fn delete(&self, file_name: &FileName) -> Result<()> {
    let file_name = file_name.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        conn.execute("DELETE FROM albums WHERE file_name = ?", params![file_name])?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete album");
        anyhow!("Failed to delete album")
      })?
  }

  #[instrument(skip_all, fields(count = file_names.len()))]
  async fn find_many(&self, file_names: Vec<FileName>) -> Result<Vec<AlbumReadModel>> {
    let album_entities = self.find_album_entities(file_names.clone()).await?;
    let album_ids = album_entities
      .values()
      .map(|album| album.id)
      .collect::<Vec<i64>>();
    let (
      mut album_artists,
      mut album_genres,
      mut album_descriptors,
      mut album_languages,
      mut album_tracks,
      mut album_credits,
      mut album_duplicates,
    ) = try_join!(
      self.find_album_artists(album_ids.clone()),
      self.find_album_genres(album_ids.clone()),
      self.find_album_descriptors(album_ids.clone()),
      self.find_album_languages(album_ids.clone()),
      self.find_album_tracks(album_ids.clone()),
      self.find_album_credits(album_ids.clone()),
      self.find_album_duplication(album_ids.clone()),
    )?;
    let mut result = Vec::<AlbumReadModel>::new();
    for file_name in file_names {
      if let Some(album_entity) = album_entities.get(&file_name) {
        let album_id = album_entity.id;
        let artists = album_artists
          .remove(&album_id)
          .unwrap_or_else(|| Vec::new());
        let (primary_genres, secondary_genres) = album_genres
          .remove(&album_id)
          .unwrap_or_else(|| (Vec::new(), Vec::new()));
        let descriptors = album_descriptors
          .remove(&album_id)
          .unwrap_or_else(|| Vec::new());
        let languages = album_languages
          .remove(&album_id)
          .unwrap_or_else(|| Vec::new());
        let tracks = album_tracks.remove(&album_id).unwrap_or_else(|| Vec::new());
        let credits = album_credits
          .remove(&album_id)
          .unwrap_or_else(|| Vec::new());
        let (duplicate_of, duplicates) = match album_duplicates
          .remove(&album_id)
          .unwrap_or_else(|| AlbumDuplication::Duplicates(Vec::new()))
        {
          AlbumDuplication::Duplicates(duplicates) => (None, duplicates),
          AlbumDuplication::DuplicateOf(duplicate_of) => (Some(duplicate_of), Vec::new()),
        };
        result.push(AlbumReadModel {
          name: album_entity.name.clone(),
          file_name: album_entity.file_name.clone(),
          rating: album_entity.rating,
          rating_count: album_entity.rating_count,
          release_date: album_entity.release_date,
          cover_image_url: album_entity.cover_image_url.clone(),
          duplicate_of,
          duplicates,
          artists,
          primary_genres,
          secondary_genres,
          descriptors,
          languages,
          tracks,
          credits,
        });
      }
    }
    Ok(result)
  }

  #[instrument(skip_all, fields(count = artist_file_name.len()))]
  async fn find_artist_albums(
    &self,
    artist_file_name: Vec<FileName>,
  ) -> Result<Vec<AlbumReadModel>> {
    let artist_file_name_params = artist_file_name
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();

    let album_file_names = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT albums.file_name
          FROM album_artists
          JOIN albums ON albums.id = album_artists.album_id
          JOIN artists ON artists.id = album_artists.artist_id
          WHERE artists.file_name IN rarray(?)
          ",
        )?;
        let mut rows = stmt.query_map([Rc::new(artist_file_name_params)], |row| {
          Ok(row.get::<_, String>(0)?)
        })?;
        let mut result_set = HashSet::<FileName>::new();
        while let Some(Ok(row)) = rows.next() {
          result_set.insert(FileName::try_from(row).map_err(|e| {
            error!(message = e.to_string(), "Failed to parse album file name");
            rusqlite::Error::ExecuteReturnedResults
          })?);
        }
        Ok::<Vec<FileName>, rusqlite::Error>(result_set.into_iter().collect::<Vec<FileName>>())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to find artist albums");
        anyhow!("Failed to find artist albums")
      })??;

    self.find_many(album_file_names).await
  }

  #[instrument(skip_all, fields(file_name))]
  async fn find(&self, file_name: &FileName) -> Result<Option<AlbumReadModel>> {
    self
      .find_many(vec![file_name.clone()])
      .await
      .map(|mut albums| albums.pop())
  }

  #[instrument(skip_all)]
  async fn get_aggregated_genres(&self, limit: Option<u32>) -> Result<Vec<GenreAggregate>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT 
            g.name,
            SUM(CASE WHEN ag.is_primary THEN 1 ELSE 0 END) as primary_genre_count,
            SUM(CASE WHEN NOT ag.is_primary THEN 1 ELSE 0 END) as secondary_genre_count
          FROM genres g
          JOIN album_genres ag ON g.id = ag.genre_id
          GROUP BY g.name
          ORDER BY primary_genre_count + secondary_genre_count DESC
          LIMIT COALESCE(?, -1)
          ",
        )?;
        let genres = stmt
          .query_map([limit], |row| {
            Ok(GenreAggregate {
              name: row.get(0)?,
              primary_genre_count: row.get(1)?,
              secondary_genre_count: row.get(2)?,
            })
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<GenreAggregate>>();
        Ok(genres)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get aggregated genres");
        anyhow!("Failed to get aggregated genres")
      })?
  }

  #[instrument(skip_all)]
  async fn get_aggregated_descriptors(&self, limit: Option<u32>) -> Result<Vec<ItemAndCount>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT d.name, COUNT(*) as count
          FROM descriptors d
          JOIN album_descriptors ad ON d.id = ad.descriptor_id
          GROUP BY d.name
          ORDER BY count DESC
          LIMIT COALESCE(?, -1)
          ",
        )?;
        let descriptors = stmt
          .query_map([limit], |row| {
            Ok(ItemAndCount {
              name: row.get(0)?,
              count: row.get(1)?,
            })
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<ItemAndCount>>();
        Ok(descriptors)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to get aggregated descriptors"
        );
        anyhow!("Failed to get aggregated descriptors")
      })?
  }

  #[instrument(skip_all)]
  async fn get_aggregated_languages(&self, limit: Option<u32>) -> Result<Vec<ItemAndCount>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT l.name, COUNT(*) as count
          FROM languages l
          JOIN album_languages al ON l.id = al.language_id
          GROUP BY l.name
          ORDER BY count DESC
          LIMIT COALESCE(?, -1)
          ",
        )?;
        let languages = stmt
          .query_map([limit], |row| {
            Ok(ItemAndCount {
              name: row.get(0)?,
              count: row.get(1)?,
            })
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<ItemAndCount>>();
        Ok(languages)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to get aggregated languages"
        );
        anyhow!("Failed to get aggregated languages")
      })?
  }

  #[instrument(skip_all)]
  async fn get_aggregated_years(&self, limit: Option<u32>) -> Result<Vec<ItemAndCount>> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT 
            strftime('%Y', release_date) AS release_year, 
            COUNT(*) AS album_count
          FROM albums
          GROUP BY release_year
          ORDER BY release_year DESC
          LIMIT COALESCE(?, -1)
          ",
        )?;
        let years = stmt
          .query_map([limit], |row| {
            Ok(ItemAndCount {
              name: row.get(0)?,
              count: row.get(1)?,
            })
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<ItemAndCount>>();
        Ok(years)
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get aggregated years");
        anyhow!("Failed to get aggregated years")
      })?
  }

  #[instrument(skip_all)]
  async fn get_album_count(&self) -> Result<u32> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM albums")?;
        Ok(
          stmt
            .query_row([], |row| row.get::<_, u32>(0))
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to get album count");
              anyhow!("Failed to get album count")
            })?,
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get album count");
        anyhow!("Failed to get album count")
      })?
  }

  #[instrument(skip_all)]
  async fn get_artist_count(&self) -> Result<u32> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM artists")?;
        Ok(
          stmt
            .query_row([], |row| row.get::<_, u32>(0))
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to get artist count");
              anyhow!("Failed to get artist count")
            })?,
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get artist count");
        anyhow!("Failed to get artist count")
      })?
  }

  #[instrument(skip_all)]
  async fn get_genre_count(&self) -> Result<u32> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM genres")?;
        Ok(
          stmt
            .query_row([], |row| row.get::<_, u32>(0))
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to get genre count");
              anyhow!("Failed to get genre count")
            })?,
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get genre count");
        anyhow!("Failed to get genre count")
      })?
  }

  #[instrument(skip_all)]
  async fn get_descriptor_count(&self) -> Result<u32> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM descriptors")?;
        Ok(
          stmt
            .query_row([], |row| row.get::<_, u32>(0))
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to get descriptor count");
              anyhow!("Failed to get descriptor count")
            })?,
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get descriptor count");
        anyhow!("Failed to get descriptor count")
      })?
  }

  #[instrument(skip_all)]
  async fn get_language_count(&self) -> Result<u32> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM languages")?;
        Ok(
          stmt
            .query_row([], |row| row.get::<_, u32>(0))
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to get language count");
              anyhow!("Failed to get language count")
            })?,
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get language count");
        anyhow!("Failed to get language count")
      })?
  }

  #[instrument(skip_all)]
  async fn get_duplicate_count(&self) -> Result<u32> {
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
            SELECT COUNT(*) FROM album_duplicates
            ",
        )?;
        Ok(
          stmt
            .query_row([], |row| row.get::<_, u32>(0))
            .map_err(|e| {
              error!(message = e.to_string(), "Failed to get duplicate count");
              anyhow!("Failed to get duplicate count")
            })?,
        )
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to get duplicate count");
        anyhow!("Failed to get duplicate count")
      })?
  }
}
