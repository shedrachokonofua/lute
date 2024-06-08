use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use async_stream::try_stream;
use chrono::{Duration, NaiveDateTime};
use derive_builder::Builder;
use futures::stream::Stream;
use rusqlite::{params, types::Value, ToSql};
use serde::{de::DeserializeOwned, Serialize};
use std::{cmp::min, collections::HashMap, rc::Rc, sync::Arc};
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

  #[instrument(skip(self, entries))]
  pub async fn put_many<T: Serialize + Send + Sync>(
    &self,
    collection: &str,
    entries: Vec<(&str, T, Option<Duration>)>,
  ) -> Result<()> {
    let entries = entries
      .into_iter()
      .map(|(key, document, ttl)| {
        let expires_at = ttl.map(|ttl| chrono::Utc::now().naive_utc() + ttl);
        let json = serde_json::to_string(&document)?;
        Ok((key.to_string(), json, expires_at))
      })
      .collect::<Result<Vec<(String, String, Option<NaiveDateTime>)>>>()?;
    let collection = collection.to_string();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        for (key, json, expires_at) in entries.into_iter() {
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
        }
        tx.commit()
          .map_err(|e| {
            error!(
              message = e.to_string(),
              "Failed to put documents in sqlite database"
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
          "Failed to put documents in sqlite database"
        );
        anyhow!("Failed to put documents in sqlite database")
      })??;
    Ok(())
  }

  #[instrument(skip(self, document))]
  pub async fn put<T: Serialize + Send + Sync>(
    &self,
    collection: &str,
    key: &str,
    document: T,
    ttl: Option<Duration>,
  ) -> Result<()> {
    self.put_many(collection, vec![(key, document, ttl)]).await
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

  pub async fn stream_index<'a, T: DeserializeOwned + Send + Sync + 'a>(
    &'a self,
    collection: &'a str,
    index: &'a str,
    start_cursor: DocumentIndexReadCursor,
    max_results: Option<usize>,
  ) -> impl Stream<Item = Result<DocumentIndexReadResult<T>>> + 'a {
    try_stream! {
      let mut returned = 0;
      let mut cursor = start_cursor.clone();
      loop {
        if max_results.is_some_and(|mr| returned >= mr) {
          break;
        }

        cursor.limit = max_results.map(|mr| min(
          mr - returned,
          start_cursor.limit,
        )).unwrap_or(cursor.limit);

        let res = self
        .read_index::<T>(collection, index, cursor.clone())
        .await?;

       let next_id_cursor = res.next_id_cursor.clone();
       let range_size = res.range_size;
        returned += res.documents.len();
        yield res;

        if let Some(next_id_cursor) = next_id_cursor {
          cursor.id_cursor = Some(next_id_cursor);
        } else {
          break;
        }

        if returned >= range_size {
          break;
        }
      }
    }
  }

  #[instrument(skip(self))]
  pub async fn count_many_by_index_key(
    &self,
    collection: &str,
    index: &str,
    index_key: Vec<String>,
  ) -> Result<HashMap<String, usize>> {
    let collection = collection.to_string();
    let index = index.to_string();
    let index_key: Vec<Value> = index_key
      .into_iter()
      .map(|k| Value::from(k.to_string()))
      .collect::<Vec<Value>>();
    let counts = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          format!(
            "
            SELECT jsonb_extract(json, '$.{}'), COUNT(*)
            FROM document_store
            WHERE collection = ?
            AND jsonb_extract(json, '$.{}') IN rarray(?)
            AND expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP
            GROUP BY jsonb_extract(json, '$.{}');
            ",
            index, index, index
          )
          .as_str(),
        )?;
        let rows = stmt.query_map(params![collection, Rc::new(index_key)], |row| {
          Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?;
        let rows = rows.collect::<Result<Vec<_>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to count many by index key in sqlite database"
        );
        anyhow!("Failed to count many by index key in sqlite database")
      })??;
    Ok(counts.into_iter().collect::<HashMap<String, usize>>())
  }

  #[instrument(skip(self))]
  pub async fn count_by_index_key(
    &self,
    collection: &str,
    index: &str,
    index_key: &str,
  ) -> Result<usize> {
    Ok(
      self
        .count_many_by_index_key(collection, index, vec![index_key.to_string()])
        .await?
        .remove(index_key)
        .unwrap_or(0),
    )
  }

  #[instrument(skip(self))]
  pub async fn find_many<T: DeserializeOwned + Send + Sync>(
    &self,
    collection: &str,
    keys: Vec<String>,
  ) -> Result<HashMap<String, Document<T>>> {
    let collection = collection.to_string();
    let keys: Vec<Value> = keys
      .into_iter()
      .map(|k| Value::from(k.to_string()))
      .collect::<Vec<Value>>();
    let documents = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut stmt = conn.prepare(
          "
          SELECT id, collection, key, json(json), created_at, updated_at, expires_at
          FROM document_store
          WHERE collection = ? AND key IN rarray(?)
          AND expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP;
          ",
        )?;
        let rows = stmt.query_map(params![collection, Rc::new(keys)], |row| {
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
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to get document from sqlite database"
        );
        anyhow!("Failed to get document from sqlite database")
      })??
      .into_iter()
      .filter_map(
        |(id, collection, key, json, created_at, updated_at, expires_at)| {
          serde_json::from_str::<T>(&json)
            .inspect_err(|e| error!(err = e.to_string(), "Failed to deserialize document"))
            .ok()
            .map(|document| {
              (
                key.clone(),
                Document {
                  id,
                  collection,
                  key,
                  document,
                  created_at,
                  updated_at,
                  expires_at,
                },
              )
            })
        },
      )
      .collect::<HashMap<String, Document<T>>>();
    Ok(documents)
  }

  #[instrument(skip(self))]
  pub async fn find<T: DeserializeOwned + Send + Sync>(
    &self,
    collection: &str,
    key: &str,
  ) -> Result<Option<Document<T>>> {
    Ok(
      self
        .find_many(collection, vec![key.to_string()])
        .await?
        .remove(key),
    )
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
