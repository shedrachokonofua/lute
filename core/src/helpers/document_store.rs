use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveDateTime};
use derive_builder::Builder;
use rusqlite::{named_params, params, OptionalExtension, ToSql};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, instrument};

#[derive(Debug, Clone)]
pub enum DocumentReadDirection {
  Forward,
  Backward,
}

#[derive(Builder, Debug, Clone)]
pub struct DocumentIndexReadCursor {
  #[builder(setter(into))]
  pub start_key: String,
  #[builder(setter(into), default = "None")]
  pub id_cursor: Option<u64>,
  #[builder(default = "100")]
  pub limit: usize,
  #[builder(default = "DocumentReadDirection::Forward")]
  pub direction: DocumentReadDirection,
  // Optional key for range queries. This is the index key to stop at
  #[builder(setter(into), default = "self.start_key.clone().unwrap()")]
  pub stop_key: String,
  // If true, the stop key is inclusive, meaning the document with the stop key will be included in the results
  #[builder(default = "true")]
  pub stop_key_inclusive: bool,
}

#[derive(Debug)]
pub struct DocumentIndexReadResult<T> {
  pub documents: Vec<Document<T>>,
  pub next_id_cursor: Option<u64>,
  pub range_size: usize,
}

#[derive(Debug)]
pub struct Document<T> {
  pub id: u64,
  pub collection: String,
  pub key: String,
  pub document: T,
  pub created_at: NaiveDateTime,
  pub updated_at: NaiveDateTime,
  pub expires_at: Option<NaiveDateTime>,
}

/**
 * DocumentStore is a lightweight helper for interacting with jsonb documents in the sqlite database
 * as if it were a document store. This is for simple use cases where a rigid relational schema is
 * not wanted and advanced querying or search capabilities are not needed. In those cases, using
 * a sqlite table or elasticsearch would be more appropriate.
 */
#[derive(Debug, Clone)]
pub struct DocumentStore {
  sqlite_connection: Arc<SqliteConnection>,
}

impl DocumentStore {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(skip(self))]
  pub async fn setup_indexes(
    &self,
    mappings: HashMap<&'static str, Vec<&'static str>>,
  ) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        for (collection, keys) in mappings.into_iter() {
          for key in keys.into_iter() {
            let index_name = format!("{}_{}_index", collection, key.replace(".", "_"));
            tx.execute(
              format!(
                "
                CREATE INDEX IF NOT EXISTS {}
                ON document_store (jsonb_extract(json, '$.{}'), id)
                WHERE collection = '{}';
                ",
                index_name, key, collection
              )
              .as_str(),
              [],
            )?;
          }
        }
        tx.commit()?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to setup indexes in sqlite database"
        );
        anyhow!("Failed to setup indexes in sqlite database")
      })?
  }

  #[instrument(skip(self, document))]
  pub async fn put<T: Serialize + Send + Sync>(
    &self,
    collection: &str,
    key: &str,
    document: T,
    ttl: Option<Duration>,
  ) -> Result<()> {
    let expires_at = ttl.map(|ttl| chrono::Utc::now().naive_utc() + ttl);
    let json = serde_json::to_string(&document)?;
    let collection = collection.to_string();
    let key = key.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        tx.execute(
          "
          INSERT INTO document_store (collection, key, json, expires_at)
          VALUES (?, ?, jsonb(?), ?)
          ON CONFLICT(collection, key) DO UPDATE SET 
            json = excluded.json,
            expires_at = excluded.expires_at,
            updated_at = CURRENT_TIMESTAMP;
          ",
          params![collection, key, json, expires_at],
        )?;
        tx.commit()
          .map_err(|e| {
            error!(
              message = e.to_string(),
              "Failed to put document in sqlite database"
            );
            e
          })
          .map_err(rusqlite::Error::from)?;
        Ok::<_, rusqlite::Error>(())
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to put document in sqlite database"
        );
        anyhow!("Failed to put document in sqlite database")
      })??;
    Ok(())
  }

  #[instrument(skip(self))]
  pub async fn read_index<T: DeserializeOwned + Send + Sync>(
    &self,
    collection: &str,
    index: &str,
    cursor: DocumentIndexReadCursor,
  ) -> Result<DocumentIndexReadResult<T>> {
    let collection = collection.to_string();
    let index = index.to_string();
    let limit = cursor.limit;
    let results = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let is_single_key = cursor.start_key == cursor.stop_key;
        let start_clause = format!(
          "AND jsonb_extract(json, '$.{}') {} :start_key",
          index,
          if is_single_key { "=" } else { ">=" }
        );
        let end_clause = if is_single_key {
          "".to_string()
        } else {
          format!(
            "AND jsonb_extract(json, '$.{}') {} :stop_key",
            index,
            if cursor.stop_key_inclusive { "<=" } else { "<" }
          )
        };
        let mut count_params: Vec<(&str, &dyn ToSql)> = vec![
          (":collection", &collection),
          (":start_key", &cursor.start_key),
        ];
        if !is_single_key {
          count_params.push((":stop_key", &cursor.stop_key));
        }
        let count = conn.query_row(
          format!(
            "
            SELECT COUNT(*)
            FROM document_store
            WHERE collection = :collection
            {}
            {}
            AND expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP;
            ",
            start_clause, end_clause
          )
          .as_str(),
          count_params.as_slice(),
          |row| row.get::<_, usize>(0),
        )?;

        let id_clause = match &cursor.id_cursor {
          Some(_) => format!(
            "AND id {} :id_cursor",
            match cursor.direction {
              DocumentReadDirection::Forward => ">=",
              DocumentReadDirection::Backward => "<=",
            }
          ),
          None => "".to_string(),
        };
        let extended_limit = limit + 1;
        let mut stmt_params: Vec<(&str, &dyn ToSql)> = vec![
          (":collection", &collection),
          (":start_key", &cursor.start_key),
          (":limit", &extended_limit),
        ];
        if let Some(id_cursor) = cursor.id_cursor.as_ref() {
          stmt_params.push((":id_cursor", id_cursor));
        }
        if !is_single_key {
          stmt_params.push((":stop_key", &cursor.stop_key));
        }
        let direction = match cursor.direction {
          DocumentReadDirection::Forward => "ASC",
          DocumentReadDirection::Backward => "DESC",
        };
        let mut stmt = conn.prepare(
          format!(
            "
            SELECT id, collection, key, json(json), created_at, updated_at, expires_at
            FROM document_store
            WHERE collection = :collection
            {}
            {}
            {}
            AND expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP
            ORDER BY jsonb_extract(json, '$.{}') {}, id {}
            LIMIT :limit;
            ",
            start_clause, end_clause, id_clause, index, direction, direction
          )
          .as_str(),
        )?;
        let rows = stmt.query_map(stmt_params.as_slice(), |row| {
          Ok((
            row.get::<_, u64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, NaiveDateTime>(4)?,
            row.get::<_, NaiveDateTime>(5)?,
            row.get::<_, Option<NaiveDateTime>>(6)?,
          ))
        })?;
        let rows = rows.collect::<Result<Vec<_>, _>>()?;
        Ok::<_, rusqlite::Error>((count, rows))
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to read index from sqlite database"
        );
        anyhow!("Failed to read index from sqlite database")
      })??;
    let (range_size, rows) = results;
    let mut documents = rows
      .into_iter()
      .filter_map(
        |(id, collection, key, json, created_at, updated_at, expires_at)| {
          serde_json::from_str::<T>(&json)
            .inspect_err(|e| error!(err = e.to_string(), "Failed to deserialize document"))
            .ok()
            .map(|document| Document {
              id,
              collection,
              key,
              document,
              created_at,
              updated_at,
              expires_at,
            })
        },
      )
      .collect::<Vec<_>>();
    let next_cursor_doc = if documents.len() > limit {
      documents.pop()
    } else {
      None
    };
    let result = DocumentIndexReadResult {
      documents,
      range_size,
      next_id_cursor: next_cursor_doc.map(|doc| doc.id),
    };
    Ok(result)
  }

  #[instrument(skip(self))]
  pub async fn count_by_index_key(
    &self,
    collection: &str,
    index: &str,
    index_key: &str,
  ) -> Result<usize> {
    let collection = collection.to_string();
    let index = index.to_string();
    let index_key = index_key.to_string();
    let count = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let count = conn.query_row(
          format!(
            "
            SELECT COUNT(*)
            FROM document_store
            WHERE collection = :collection
            AND jsonb_extract(json, '$.{}') = :index_key
            AND expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP;
            ",
            index
          )
          .as_str(),
          named_params! {
            ":collection": collection,
            ":index_key": index_key,
          },
          |row| row.get::<_, i64>(0),
        )?;
        Ok::<_, rusqlite::Error>(count as usize)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to count by index key in sqlite database"
        );
        anyhow!("Failed to count by index key in sqlite database")
      })??;
    Ok(count)
  }

  #[instrument(skip(self))]
  pub async fn get<T: DeserializeOwned + Send + Sync>(
    &self,
    collection: &str,
    key: &str,
  ) -> Result<Option<Document<T>>> {
    let collection = collection.to_string();
    let key = key.to_string();
    let document = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        conn
          .query_row(
            "
            SELECT id, collection, key, json(json), created_at, updated_at, expires_at
            FROM document_store
            WHERE collection = ? AND key = ? AND expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP;
            ",
            params![collection, key],
            |row| {
              Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, NaiveDateTime>(4)?,
                row.get::<_, NaiveDateTime>(5)?,
                row.get::<_, Option<NaiveDateTime>>(6)?,
              ))
            },
          )
          .optional()
          .map_err(|e| {
            error!(message = e.to_string(), "Failed to get key value");
            rusqlite::Error::ExecuteReturnedResults
          })
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to get document from sqlite database"
        );
        anyhow!("Failed to get document from sqlite database")
      })??
      .and_then(
        |(id, collection, key, json, created_at, updated_at, expires_at)| {
          serde_json::from_str::<T>(&json)
            .inspect_err(|e| error!(err = e.to_string(), "Failed to deserialize document"))
            .ok()
            .map(|document| Document {
              id,
              collection,
              key,
              document,
              created_at,
              updated_at,
              expires_at,
            })
        },
      );
    Ok(document)
  }

  #[instrument(skip(self))]
  pub async fn delete(&self, collection: &str, key: &str) -> Result<()> {
    let collection = collection.to_string();
    let key = key.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        tx.execute(
          "
          DELETE FROM document_store
          WHERE collection = ? AND key = ?;
          ",
          params![collection, key],
        )?;
        tx.commit()?;
        Ok::<_, rusqlite::Error>(())
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to delete document from sqlite database"
        );
        anyhow!("Failed to delete document from sqlite database")
      })??;
    Ok(())
  }
}
