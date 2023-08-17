// @generated automatically by Diesel CLI.

diesel::table! {
    lute_artists (id) {
        id -> Int4,
        file_name -> Varchar,
        name -> Varchar,
    }
}

diesel::table! {
    lute_events (id) {
        id -> Varchar,
        stream_id -> Varchar,
        payload -> Jsonb,
        event_timestamp -> Int8,
        saved_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    lute_artists,
    lute_events,
);
