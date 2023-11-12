use crate::settings::Settings;
use anyhow::Result;
use deadpool_sqlite::{Config, Hook, HookError, Object, Pool, Runtime};
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use rusqlite::vtab;
use rusqlite_migration::Migrations;
use std::{path::Path, sync::Arc};
use tracing::{error, info};

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

lazy_static! {
  static ref MIGRATIONS: Migrations<'static> = Migrations::from_directory(&MIGRATIONS_DIR).unwrap();
}

#[derive(Clone, Debug)]
pub struct SqliteConnection {
  pool: Arc<Pool>,
}

impl SqliteConnection {
  pub async fn new(settings: Arc<Settings>) -> Result<Self> {
    let config = Config::new(Path::new(&settings.sqlite.dir).join("lute.db"));
    let pool = config
      .builder(Runtime::Tokio1)?
      .post_create(Hook::async_fn(|wrapper, _| {
        Box::pin(async move {
          wrapper
            .interact(|conn| {
              conn.pragma_update(None, "journal_mode", "WAL")?;
              conn.pragma_update(None, "foreign_keys", "ON")?;
              conn.pragma_update(None, "synchronous", "NORMAL")?;
              vtab::array::load_module(&conn)?;
              info!("Sqlite connection initialized");
              Ok::<_, rusqlite::Error>(())
            })
            .await
            .map_err(|e| {
              error!("Failed to initialize SQLite connection: {:?}", e);
              HookError::Message(format!("Failed to initialize SQLite connection: {:?}", e))
            })?
            .map_err(|e| {
              error!("Failed to initialize SQLite connection: {:?}", e);
              HookError::Message(format!("Failed to initialize SQLite connection: {:?}", e))
            })
        })
      }))
      .build()
      .map_err(|e| {
        error!("Failed to initialize SQLite connection: {:?}", e);
        anyhow::anyhow!("Failed to initialize SQLite connection: {:?}", e)
      })?;

    let sqlite_connection = Self {
      pool: Arc::new(pool),
    };
    sqlite_connection.migrate_to_latest().await?;

    Ok(sqlite_connection)
  }

  pub async fn migrate_to_latest(&self) -> Result<()> {
    self
      .pool
      .get()
      .await?
      .interact(|conn| {
        MIGRATIONS.to_latest(conn)?;
        info!("Sqlite database migrated to latest version");
        Ok(())
      })
      .await
      .map_err(|e| {
        error!("Failed to migrate SQLite database: {:?}", e);
        anyhow::anyhow!("Failed to migrate SQLite database: {:?}", e)
      })?
  }

  pub async fn migrate_to_version(&self, version: u32) -> Result<()> {
    self
      .pool
      .get()
      .await?
      .interact(move |conn| {
        MIGRATIONS.to_version(conn, version as usize)?;
        Ok(())
      })
      .await
      .map_err(|e| {
        error!("Failed to migrate SQLite database: {:?}", e);
        anyhow::anyhow!("Failed to migrate SQLite database: {:?}", e)
      })?
  }

  pub async fn get(&self) -> Result<Object> {
    self.pool.get().await.map_err(|e| {
      error!("Failed to get SQLite connection: {:?}", e);
      anyhow::anyhow!("Failed to get SQLite connection: {:?}", e)
    })
  }
}
