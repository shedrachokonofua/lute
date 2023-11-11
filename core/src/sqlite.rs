use crate::settings::Settings;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use rusqlite::vtab;
use rusqlite_migration::AsyncMigrations;
use std::{path::Path, sync::Arc};
use tokio_rusqlite::Connection;
use tracing::info;

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

lazy_static! {
  pub static ref MIGRATIONS: AsyncMigrations =
    AsyncMigrations::from_directory(&MIGRATIONS_DIR).unwrap();
}

pub async fn build_sqlite_connection(settings: Arc<Settings>) -> Result<Connection> {
  let connection = Connection::open(Path::new(&settings.sqlite.dir).join("lute.db")).await?;
  connection
    .call(|conn| {
      conn.pragma_update(None, "journal_mode", "WAL")?;
      conn.pragma_update(None, "foreign_keys", "ON")?;
      conn.pragma_update(None, "synchronous", "NORMAL")?;
      vtab::array::load_module(&conn)?;
      Ok(())
    })
    .await?;
  Ok(connection)
}

pub async fn migrate_to_latest(settings: Arc<Settings>) -> Result<()> {
  let mut connection = build_sqlite_connection(settings).await?;
  MIGRATIONS.to_latest(&mut connection).await?;
  Ok(())
}

pub async fn migrate_to_version(settings: Arc<Settings>, version: u32) -> Result<()> {
  let mut connection = build_sqlite_connection(settings).await?;
  MIGRATIONS
    .to_version(&mut connection, version as usize)
    .await?;
  Ok(())
}

pub async fn connect_to_sqlite(settings: Arc<Settings>) -> Result<Arc<tokio_rusqlite::Connection>> {
  let connection = Arc::new(build_sqlite_connection(settings.clone()).await?);
  info!("Connected to SQLite database");
  migrate_to_latest(Arc::clone(&settings)).await?;
  info!("All sqlite migrations applied");
  Ok(connection)
}

#[derive(Clone, Debug)]
pub struct SqliteConnection {
  read_connection: Arc<tokio_rusqlite::Connection>,
  write_connection: Arc<tokio_rusqlite::Connection>,
}

impl SqliteConnection {
  pub async fn new(settings: Arc<Settings>) -> Result<Self> {
    let read_connection = Arc::new(build_sqlite_connection(settings.clone()).await?);
    let write_connection = Arc::new(build_sqlite_connection(settings.clone()).await?);
    migrate_to_latest(Arc::clone(&settings)).await?;
    Ok(Self {
      read_connection,
      write_connection,
    })
  }

  pub fn read(&self) -> Arc<tokio_rusqlite::Connection> {
    Arc::clone(&self.read_connection)
  }

  pub fn write(&self) -> Arc<tokio_rusqlite::Connection> {
    Arc::clone(&self.write_connection)
  }
}
