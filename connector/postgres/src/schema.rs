// @generated automatically by Diesel CLI.

diesel::table! {
    lute_albums (file_name) {
        file_name -> Text,
        name -> Text,
        rating -> Float8,
        rating_count -> Int4,
        primary_genres -> Array<Nullable<Text>>,
        secondary_genres -> Array<Nullable<Text>>,
        descriptors -> Array<Nullable<Text>>,
        release_date -> Nullable<Date>,
        languages -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    lute_albums_artists (artist_file_name, album_file_name) {
        artist_file_name -> Text,
        album_file_name -> Text,
    }
}

diesel::table! {
    lute_artists (file_name) {
        file_name -> Text,
        name -> Text,
    }
}

diesel::table! {
    lute_credits (artist_file_name, album_file_name) {
        artist_file_name -> Text,
        album_file_name -> Text,
        roles -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    lute_events (id) {
        id -> Int4,
        entry_id -> Varchar,
        stream_id -> Varchar,
        payload -> Jsonb,
        event_timestamp -> Int8,
        saved_at -> Timestamp,
    }
}

diesel::table! {
    lute_tracks (album_file_name, name) {
        album_file_name -> Text,
        name -> Text,
        duration_seconds -> Nullable<Int4>,
        rating -> Nullable<Float8>,
        position -> Nullable<Text>,
    }
}

diesel::joinable!(lute_albums_artists -> lute_albums (album_file_name));
diesel::joinable!(lute_albums_artists -> lute_artists (artist_file_name));
diesel::joinable!(lute_credits -> lute_albums (album_file_name));
diesel::joinable!(lute_credits -> lute_artists (artist_file_name));
diesel::joinable!(lute_tracks -> lute_albums (album_file_name));

diesel::allow_tables_to_appear_in_same_query!(
    lute_albums,
    lute_albums_artists,
    lute_artists,
    lute_credits,
    lute_events,
    lute_tracks,
);
