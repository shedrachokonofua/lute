use crate::settings::Settings;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite_migration::Migrations;
use std::{path::Path, sync::Arc};

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");

lazy_static! {
  pub static ref MIGRATIONS: Migrations<'static> =
    Migrations::from_directory(&MIGRATIONS_DIR).unwrap();
}

pub async fn build_sqlite_connection_pool(
  settings: Arc<Settings>,
) -> Result<Pool<SqliteConnectionManager>> {
  Pool::new(
    SqliteConnectionManager::file(Path::new(&settings.sqlite.dir).join("lute.db")).with_init(|c| {
      c.pragma_update(None, "journal_mode", "WAL")?;
      c.pragma_update(None, "foreign_keys", "ON")?;
      Ok(())
    }),
  )
  .map_err(|e| e.into())
}

pub fn migrate_to_latest(pool: Arc<Pool<SqliteConnectionManager>>) -> Result<()> {
  let mut connection = pool.get()?;
  MIGRATIONS.to_latest(&mut connection)?;
  Ok(())
}

pub fn migrate_to_version(pool: Arc<Pool<SqliteConnectionManager>>, version: u32) -> Result<()> {
  let mut connection = pool.get()?;
  MIGRATIONS.to_version(&mut connection, version as usize)?;
  Ok(())
}

pub async fn connect_to_sqlite(
  settings: Arc<Settings>,
) -> Result<Arc<Pool<SqliteConnectionManager>>> {
  let pool = Arc::new(build_sqlite_connection_pool(settings.clone()).await?);
  migrate_to_latest(Arc::clone(&pool))?;
  Ok(pool)
}
