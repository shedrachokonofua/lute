// @generated automatically by Diesel CLI.

diesel::table! {
    lute_events (id) {
        id -> Varchar,
        stream_id -> Varchar,
        payload -> Jsonb,
        event_timestamp -> Int8,
        saved_at -> Timestamp,
    }
}
