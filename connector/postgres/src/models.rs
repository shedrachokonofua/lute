use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::lute_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LuteEvent {
  pub id: String,
  pub stream_id: String,
  pub payload: Value,
  pub event_timestamp: i64,
  pub saved_at: NaiveDateTime,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schema::lute_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewLuteEvent {
  pub id: String,
  pub stream_id: String,
  pub payload: Value,
  pub event_timestamp: i64,
}
