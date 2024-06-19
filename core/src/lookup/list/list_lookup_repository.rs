use crate::{
  files::file_metadata::{
    file_name::{FileName, ListRootFileName},
    page_type::PageType,
  },
  lookup::ListLookupStatus,
  parser::parsed_file_data::ParsedListSegment,
  sqlite::SqliteConnection,
};
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use rusqlite::{params, types::Value};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc, sync::Arc};
use tokio::try_join;
use tracing::error;

#[derive(Serialize, Deserialize, Clone)]
pub struct ListSegmentReadModel {
  pub file_name: FileName,
  pub root_file_name: ListRootFileName,
  pub other_segments: Vec<FileName>,
  pub albums: Vec<FileName>,
}

struct ListSegmentRecord {
  pub id: i64,
  pub file_name: String,
}

struct ListSegmentSiblingRecord {
  pub list_segment_id: i64,
  pub sibling_file_name: String,
}

struct ListSegmentAlbumRecord {
  pub list_segment_id: i64,
  pub file_name: String,
}

pub struct ListLookupRecord {
  pub root_file_name: ListRootFileName,
  pub latest_status: ListLookupStatus,
  pub latest_run: Option<NaiveDateTime>,
}

impl ListSegmentReadModel {
  pub fn try_from_parsed_list_segment(
    file_name: FileName,
    data: ParsedListSegment,
  ) -> Result<Self> {
    Ok(Self {
      root_file_name: ListRootFileName::try_from(file_name.clone())?,
      file_name,
      other_segments: data.other_segments,
      albums: data.albums,
    })
  }
}

pub struct ListLookupRepository {
  sqlite_connection: Arc<SqliteConnection>,
}

impl ListLookupRepository {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  pub async fn put_many_segments(&self, updates: Vec<ListSegmentReadModel>) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        for segment in updates {
          let segment_id: i64 = tx.query_row(
            "
            INSERT INTO list_segments (file_name, root_file_name)
            VALUES (?, ?)
            ON CONFLICT (file_name) DO UPDATE SET root_file_name = excluded.root_file_name
            RETURNING id
            ",
            params![
              segment.file_name.to_string(),
              segment.root_file_name.to_string()
            ],
            |row| row.get(0),
          )?;
          for sibling in segment.other_segments {
            tx.execute(
              "
              INSERT OR IGNORE INTO list_segment_siblings (list_segment_id, sibling_file_name)
              VALUES (?, ?)
              ",
              params![segment_id, sibling.to_string()],
            )?;
          }
          for album in segment.albums {
            tx.execute(
              "
              INSERT OR IGNORE INTO list_segment_albums (list_segment_id, file_name)
              VALUES (?, ?)
              ",
              params![segment_id, album.to_string()],
            )?;
          }
        }
        tx.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to put segments");
        anyhow!("Failed to put segments")
      })?
  }

  async fn find_many_segment_records(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<Vec<ListSegmentRecord>> {
    let values = file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare_cached(
          "
          SELECT id, file_name
          FROM list_segments
          WHERE file_name IN rarray(?)
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(values)], |row| {
            Ok(ListSegmentRecord {
              id: row.get(0)?,
              file_name: row.get(1)?,
            })
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<ListSegmentRecord>>();
        Ok(rows)
      })
      .await
      .inspect_err(|e| {
        error!(message = e.to_string(), "Failed to find records");
      })
      .map_err(|e| anyhow!("Failed to find records {}", e))?
  }

  async fn find_many_segment_sibling_records(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<Vec<ListSegmentSiblingRecord>> {
    let values = file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare_cached(
          "
          SELECT sib.list_segment_id, sib.sibling_file_name
          FROM list_segment_siblings sib
          JOIN list_segments ON list_segments.id = sib.list_segment_id
          WHERE list_segments.file_name IN rarray(?)
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(values)], |row| {
            Ok(ListSegmentSiblingRecord {
              list_segment_id: row.get(0)?,
              sibling_file_name: row.get(1)?,
            })
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<ListSegmentSiblingRecord>>();
        Ok(rows)
      })
      .await
      .inspect_err(|e| {
        error!(message = e.to_string(), "Failed to find records");
      })
      .map_err(|e| anyhow!("Failed to find records {}", e))?
  }

  async fn find_many_segment_album_records(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<Vec<ListSegmentAlbumRecord>> {
    let values = file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();
    self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare_cached(
          "
          SELECT al.list_segment_id, al.file_name
          FROM list_segment_albums al
          JOIN list_segments ON list_segments.id = al.list_segment_id
          WHERE list_segments.file_name IN rarray(?)
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(values)], |row| {
            Ok(ListSegmentAlbumRecord {
              list_segment_id: row.get(0)?,
              file_name: row.get(1)?,
            })
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<ListSegmentAlbumRecord>>();
        Ok(rows)
      })
      .await
      .inspect_err(|e| {
        error!(message = e.to_string(), "Failed to find records");
      })
      .map_err(|e| anyhow!("Failed to find records {}", e))?
  }

  pub async fn find_many_segments(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<Vec<ListSegmentReadModel>> {
    let (mut segment_records, mut sibling_records, mut album_records) = try_join!(
      self.find_many_segment_records(file_names.clone()),
      self.find_many_segment_sibling_records(file_names.clone()),
      self.find_many_segment_album_records(file_names)
    )?;

    let mut segments = HashMap::new();
    for segment in segment_records.drain(..) {
      let file_name = FileName::try_from(segment.file_name)?;
      segments.insert(
        segment.id,
        ListSegmentReadModel {
          root_file_name: ListRootFileName::try_from(file_name.clone())?,
          other_segments: vec![],
          albums: vec![],
          file_name,
        },
      );
    }
    for sibling in sibling_records.drain(..) {
      if let Some(segment) = segments.get_mut(&sibling.list_segment_id) {
        segment
          .other_segments
          .push(FileName::try_from(sibling.sibling_file_name)?);
      }
    }
    for album in album_records.drain(..) {
      if let Some(segment) = segments.get_mut(&album.list_segment_id) {
        segment.albums.push(FileName::try_from(album.file_name)?);
      }
    }

    Ok(segments.into_iter().map(|(_, v)| v).collect())
  }

  pub async fn find_many_segments_by_root(
    &self,
    root_file_name: Vec<ListRootFileName>,
  ) -> Result<HashMap<ListRootFileName, Vec<ListSegmentReadModel>>> {
    let values = root_file_name
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();
    let segment_file_names = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare_cached(
          "
          SELECT file_name
          FROM list_segments
          WHERE root_file_name IN rarray(?)
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(values)], |row| Ok(row.get(0)?))?
          .filter_map(|r| r.ok())
          .collect::<Vec<String>>();
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .inspect_err(|e| {
        error!(message = e.to_string(), "Failed to find records");
      })
      .map_err(|e| anyhow!("Failed to find records {}", e))??
      .into_iter()
      .map(FileName::try_from)
      .collect::<Result<Vec<FileName>>>()?;

    let segments = self.find_many_segments(segment_file_names).await?;

    let mut results = HashMap::new();
    for segment in segments {
      results
        .entry(segment.root_file_name.clone())
        .or_insert_with(Vec::new)
        .push(segment);
    }

    Ok(results)
  }

  async fn find_lookups_containing_siblings(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<Vec<ListLookupRecord>> {
    let values = file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();
    let result = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT root_file_name, latest_status, latest_run
          FROM list_lookups
          WHERE root_file_name IN (
            SELECT DISTINCT l.root_file_name
            FROM list_lookups l
            JOIN list_segments ON list_segments.root_file_name = l.root_file_name
            JOIN list_segment_siblings ON list_segment_siblings.list_segment_id = list_segments.id
            WHERE list_segment_siblings.sibling_file_name IN rarray(?)
          )
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(values)], |row| {
            Ok((
              row.get::<_, String>(0)?,
              row.get::<_, u32>(1)?,
              row.get::<_, Option<NaiveDateTime>>(2)?,
            ))
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<_>>();
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .inspect_err(|e| {
        error!(message = e.to_string(), "Failed to find records");
      })
      .map_err(|e| anyhow!("Failed to find records {}", e))??
      .into_iter()
      .map(|(root_file_name, latest_status, latest_run)| {
        Ok(ListLookupRecord {
          root_file_name: ListRootFileName::try_from(root_file_name)?,
          latest_status: serde_json::from_str(&latest_status.to_string())?,
          latest_run,
        })
      })
      .collect::<Result<Vec<ListLookupRecord>>>()?;
    Ok(result)
  }

  async fn find_lookups_containing_albums(
    &self,
    file_names: Vec<FileName>,
  ) -> Result<Vec<ListLookupRecord>> {
    let values = file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();
    let result = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT root_file_name, latest_status, latest_run
          FROM list_lookups
          WHERE root_file_name IN (
            SELECT DISTINCT l.root_file_name
            FROM list_lookups l
            JOIN list_segments ON list_segments.root_file_name = l.root_file_name
            JOIN list_segment_albums ON list_segment_albums.list_segment_id = list_segments.id
            WHERE list_segment_albums.file_name IN rarray(?)
          )
          ",
        )?;
        let rows = stmt
          .query_map([Rc::new(values)], |row| {
            Ok((
              row.get::<_, String>(0)?,
              row.get::<_, u32>(1)?,
              row.get::<_, Option<NaiveDateTime>>(2)?,
            ))
          })?
          .filter_map(|r| r.ok())
          .collect::<Vec<_>>();
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .inspect_err(|e| {
        error!(message = e.to_string(), "Failed to find records");
      })
      .map_err(|e| anyhow!("Failed to find records {}", e))??
      .into_iter()
      .map(|(root_file_name, latest_status, latest_run)| {
        Ok(ListLookupRecord {
          root_file_name: ListRootFileName::try_from(root_file_name)?,
          latest_status: serde_json::from_str(&latest_status.to_string())?,
          latest_run,
        })
      })
      .collect::<Result<Vec<ListLookupRecord>>>()?;
    Ok(result)
  }

  pub async fn find_lookups_containing_components(
    &self,
    components: Vec<FileName>,
  ) -> Result<Vec<ListLookupRecord>> {
    let mut sibling_values = vec![];
    let mut album_values = vec![];
    for component in components {
      match component.page_type() {
        PageType::ListSegment => sibling_values.push(component),
        PageType::Album => album_values.push(component),
        _ => {}
      }
    }

    if sibling_values.is_empty() && album_values.is_empty() {
      return Ok(vec![]);
    }

    let (sibling_results, album_results) = try_join!(
      self.find_lookups_containing_siblings(sibling_values),
      self.find_lookups_containing_albums(album_values)
    )?;

    let mut results = HashMap::new();
    for lookup in sibling_results.into_iter().chain(album_results) {
      results.insert(lookup.root_file_name.clone(), lookup);
    }

    Ok(results.into_iter().map(|(_, v)| v).collect())
  }

  pub async fn put_lookup_record(
    &self,
    root_file_name: ListRootFileName,
  ) -> Result<ListLookupRecord> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let (latest_status, latest_run) = conn.query_row(
          "
          INSERT INTO list_lookups (root_file_name)
          VALUES (?)
          ON CONFLICT (root_file_name) DO UPDATE SET root_file_name = excluded.root_file_name
          RETURNING latest_status, latest_run
          ",
          params![root_file_name.to_string()],
          |row| {
            Ok((
              row.get::<_, u32>(0)?,
              row.get::<_, Option<NaiveDateTime>>(1)?,
            ))
          },
        )?;
        Ok(ListLookupRecord {
          latest_status: serde_json::from_str(&latest_status.to_string())?,
          root_file_name,
          latest_run,
        })
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to put lookup");
        anyhow!("Failed to put lookup")
      })?
  }

  pub async fn delete_many_lookups(&self, root_file_names: Vec<ListRootFileName>) -> Result<()> {
    let values = root_file_names
      .iter()
      .map(|f| Value::from(f.to_string()))
      .collect::<Vec<Value>>();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        conn.execute(
          "DELETE FROM list_lookups WHERE root_file_name IN rarray(?)",
          [Rc::new(values)],
        )?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to delete lookup");
        anyhow!("Failed to delete lookup")
      })?
  }

  pub async fn update_many_lookup_records(
    &self,
    updates: Vec<(ListRootFileName, ListLookupStatus, Option<NaiveDateTime>)>,
  ) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        for (root_file_name, status, latest_run) in updates {
          tx.execute(
            "UPDATE list_lookups SET latest_status = ? WHERE root_file_name = ?",
            params![
              serde_json::to_string(&status).unwrap(),
              root_file_name.to_string()
            ],
          )?;
          if let Some(latest_run) = latest_run {
            tx.execute(
              "UPDATE list_lookups SET latest_run = ? WHERE root_file_name = ?",
              params![latest_run, root_file_name.to_string()],
            )?;
          }
        }
        tx.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(message = e.to_string(), "Failed to update lookups");
        anyhow!("Failed to update lookups")
      })?
  }
}
