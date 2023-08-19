use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use serde_json::Value;

#[derive(Queryable, Identifiable, Selectable, Debug)]
#[diesel(table_name = crate::schema::lute_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LuteEvent {
  pub id: i32,
  pub entry_id: String,
  pub stream_id: String,
  pub payload: Value,
  pub event_timestamp: i64,
  pub saved_at: NaiveDateTime,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schema::lute_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewLuteEvent {
  pub entry_id: String,
  pub stream_id: String,
  pub payload: Value,
  pub event_timestamp: i64,
}

#[derive(Queryable, Identifiable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::lute_artists)]
#[diesel(primary_key(file_name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LuteArtist {
  pub file_name: String,
  pub name: String,
}

#[derive(Queryable, Identifiable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::lute_albums)]
#[diesel(primary_key(file_name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LuteAlbum {
  pub file_name: String,
  pub name: String,
  pub rating: f64,
  pub rating_count: i32,
  pub primary_genres: Vec<Option<String>>,
  pub secondary_genres: Vec<Option<String>>,
  pub descriptors: Vec<Option<String>>,
  pub languages: Vec<Option<String>>,
  pub release_date: Option<NaiveDate>,
}

#[derive(Queryable, Selectable, Associations, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::lute_albums_artists)]
#[diesel(belongs_to(LuteAlbum, foreign_key = album_file_name))]
#[diesel(belongs_to(LuteArtist, foreign_key = artist_file_name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LuteAlbumArtist {
  pub album_file_name: String,
  pub artist_file_name: String,
}

#[derive(Queryable, Selectable, Associations, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::lute_tracks)]
#[diesel(belongs_to(LuteAlbum, foreign_key = album_file_name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LuteTrack {
  pub album_file_name: String,
  pub name: String,
  pub duration_seconds: Option<i32>,
  pub rating: Option<f64>,
  pub position: Option<String>,
}

#[derive(Queryable, Identifiable, Selectable, Associations, Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::lute_credits)]
#[diesel(belongs_to(LuteAlbum, foreign_key = album_file_name))]
#[diesel(belongs_to(LuteArtist, foreign_key = artist_file_name))]
#[diesel(primary_key(album_file_name, artist_file_name))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LuteCredit {
  pub album_file_name: String,
  pub artist_file_name: String,
  pub roles: Vec<Option<String>>,
}
