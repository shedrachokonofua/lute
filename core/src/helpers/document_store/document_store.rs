use super::document_filter::DocumentFilter;
use crate::sqlite::SqliteConnection;
use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveDateTime};
use rusqlite::{params, types::Value, ToSql};
use serde::{de::DeserializeOwned, Serialize};
use std::{borrow::BorrowMut, collections::HashMap, rc::Rc, sync::Arc};
use tracing::{error, instrument};

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

#[derive(Debug, Clone)]
pub struct DocumentCursor {
  pub cursor: Option<String>,
  pub limit: usize,
}

impl DocumentCursor {
  pub fn new(cursor: Option<String>, limit: usize) -> Self {
    Self { cursor, limit }
  }

  pub fn with_limit(limit: usize) -> Self {
    Self {
      cursor: None,
      limit,
    }
  }
}

#[derive(Debug)]
pub struct DocumentFindResult<T> {
  pub documents: Vec<Document<T>>,
  pub next_cursor: Option<String>,
  pub range_size: usize,
}

/**
 * DocumentStore is a lightweight helper for interacting with jsonb documents in the sqlite database
 * as if it were a document store. This is for simple use cases where a rigid relational schema is
 * not wanted and advanced querying or search capabilities are not needed. In those cases, using
 * a sqlite directly or elasticsearch would be more appropriate.
 */
#[derive(Debug, Clone)]
pub struct DocumentStore {
  sqlite_connection: Arc<SqliteConnection>,
}

impl DocumentStore {
  pub fn new(sqlite_connection: Arc<SqliteConnection>) -> Self {
    Self { sqlite_connection }
  }

  #[instrument(skip(self), name = "DocumentStore::setup_indexes")]
  pub async fn setup_indexes(
    &self,
    mappings: HashMap<&'static str, Vec<Vec<&'static str>>>,
  ) -> Result<()> {
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        for (collection, keys) in mappings.into_iter() {
          for key in keys.into_iter() {
            let index_name = format!("idx_{}_{}", collection, key.join("_").replace(".", "_"));
            let mut index_keys = vec!["collection".to_string()];
            index_keys.extend(
              key
                .into_iter()
                .map(|k| format!("jsonb_extract(json, '$.{}')", k))
                .collect::<Vec<String>>(),
            );
            index_keys.push("key".to_string());
            index_keys.push("expires_at".to_string());
            let index_keys = index_keys.join(", ");
            tx.execute(
              format!(
                "
                CREATE INDEX IF NOT EXISTS {}
                ON document_store ({})
                WHERE collection = '{}';
                ",
                index_name, index_keys, collection
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

  #[instrument(skip(self), name = "DocumentStore::find_many")]
  pub async fn find_many<T: DeserializeOwned + Send + Sync>(
    &self,
    collection: &str,
    filter: DocumentFilter,
    cursor: Option<DocumentCursor>,
  ) -> Result<DocumentFindResult<T>> {
    let collection = collection.to_string();
    let mut filter = filter;
    let (sql, params) = filter.borrow_mut().to_sql(collection.clone())?;
    let cursor_limit = cursor.as_ref().map(|c| c.limit);
    let (range_size, rows) = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let mut count_stmt = conn.prepare(
          sql
            .replace(&DocumentFilter::columns_select_list(), "COUNT(*)")
            .as_str(),
        )?;
        let mut params = params
          .iter()
          .map(|(k, v)| (k.as_ref(), v as &dyn ToSql))
          .collect::<Vec<_>>();
        let count =
          count_stmt.query_row(params.clone().as_slice(), |row| row.get::<_, usize>(0))?;

        let mut row_sql = sql;
        let cursor_key = cursor.as_ref().and_then(|c| c.cursor.clone());
        let cursor_limit = cursor.map(|c| c.limit);
        if cursor_key.is_some() {
          row_sql = format!("{} AND key > :cursor_key", row_sql);
          params.push((":cursor_key", &cursor_key as &dyn ToSql));
        }
        row_sql = format!("{} ORDER BY key ASC", row_sql);
        if cursor_limit.is_some() {
          row_sql = format!("{} LIMIT :cursor_limit", row_sql);
          params.push((":cursor_limit", &cursor_limit as &dyn ToSql));
        }

        let mut stmt = conn.prepare(&row_sql)?;
        let rows = stmt.query_map(params.as_slice(), |row| {
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
          "Failed to find many from sqlite database"
        );
        anyhow!("Failed to find many from sqlite database")
      })??;
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
    let next_cursor = if cursor_limit.is_some_and(|l| documents.len() > l) {
      documents.pop().map(|d| d.key)
    } else {
      None
    };
    let result = DocumentFindResult {
      documents,
      range_size,
      next_cursor,
    };
    Ok(result)
  }

  #[instrument(skip(self, entries), name = "DocumentStore::put_many")]
  pub async fn put_many<T: Serialize + Send + Sync>(
    &self,
    collection: &str,
    entries: Vec<(String, T, Option<Duration>)>,
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

  #[instrument(skip(self, document), name = "DocumentStore::put")]
  pub async fn put<T: Serialize + Send + Sync>(
    &self,
    collection: &str,
    key: &str,
    document: T,
    ttl: Option<Duration>,
  ) -> Result<()> {
    self
      .put_many(collection, vec![(key.to_string(), document, ttl)])
      .await
  }

  #[instrument(skip(self), name = "DocumentStore::count_each_field_value")]
  pub async fn count_each_field_value(
    &self,
    collection: &str,
    field: &str,
    filter: Option<DocumentFilter>,
  ) -> Result<HashMap<String, usize>> {
    let collection = collection.to_string();
    let field = field.to_string();
    let mut filter = filter.unwrap_or_default();
    let (sql, params) = filter.borrow_mut().to_sql(collection.clone())?;
    let counts = self
      .sqlite_connection
      .read()
      .await?
      .interact(move |conn| {
        let sql = format!(
          "
          {}
          GROUP BY jsonb_extract(json, '$.{}');
          ",
          sql.replace(
            &DocumentFilter::columns_select_list(),
            format!("jsonb_extract(json, '$.{}'), COUNT(*)", field).as_str()
          ),
          field
        );
        let params = params
          .iter()
          .map(|(k, v)| (k.as_ref(), v as &dyn ToSql))
          .collect::<Vec<_>>();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), |row| {
          Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?;
        let rows = rows.collect::<Result<Vec<_>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to count each field in sqlite database"
        );
        anyhow!("Failed to count each field in sqlite database")
      })??;
    Ok(counts.into_iter().collect::<HashMap<String, usize>>())
  }

  #[instrument(skip(self), name = "DocumentStore::count_many_by_field_value")]
  pub async fn count_many_by_field_value(
    &self,
    collection: &str,
    field: &str,
    values: Vec<String>,
  ) -> Result<HashMap<String, usize>> {
    let collection = collection.to_string();
    let field = field.to_string();
    let values: Vec<Value> = values
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
            field, field, field
          )
          .as_str(),
        )?;
        let rows = stmt.query_map(params![collection, Rc::new(values)], |row| {
          Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?;
        let rows = rows.collect::<Result<Vec<_>, _>>()?;
        Ok::<_, rusqlite::Error>(rows)
      })
      .await
      .map_err(|e| {
        error!(
          message = e.to_string(),
          "Failed to count many by field value in sqlite database"
        );
        anyhow!("Failed to count many by field value in sqlite database")
      })??;
    Ok(counts.into_iter().collect::<HashMap<String, usize>>())
  }

  #[instrument(skip(self), name = "DocumentStore::count_field_value")]
  pub async fn count_field_value(
    &self,
    collection: &str,
    field: &str,
    value: &str,
  ) -> Result<usize> {
    Ok(
      self
        .count_many_by_field_value(collection, field, vec![value.to_string()])
        .await?
        .remove(value)
        .unwrap_or(0),
    )
  }

  #[instrument(skip(self), name = "DocumentStore::find_many_by_key")]
  pub async fn find_many_by_key<T: DeserializeOwned + Send + Sync>(
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

  #[instrument(skip(self), name = "DocumentStore::find_by_key")]
  pub async fn find_by_key<T: DeserializeOwned + Send + Sync>(
    &self,
    collection: &str,
    key: &str,
  ) -> Result<Option<Document<T>>> {
    Ok(
      self
        .find_many_by_key(collection, vec![key.to_string()])
        .await?
        .remove(key),
    )
  }

  #[instrument(skip(self), name = "DocumentStore::delete_many")]
  pub async fn delete_many(&self, collection: &str, keys: Vec<String>) -> Result<()> {
    let collection = collection.to_string();
    let keys = keys.into_iter().map(Value::from).collect::<Vec<_>>();
    self
      .sqlite_connection
      .write()
      .await?
      .interact(move |conn| {
        let tx = conn.transaction()?;
        tx.execute(
          "
          DELETE FROM document_store
          WHERE collection = ? AND key IN rarray(?);
          ",
          params![collection, Rc::new(keys)],
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

  #[instrument(skip(self), name = "DocumentStore::delete")]
  pub async fn delete(&self, collection: &str, key: &str) -> Result<()> {
    self.delete_many(collection, vec![key.to_string()]).await
  }
}
