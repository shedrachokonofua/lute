CREATE INDEX idx_duplicates_duplicate_album_id ON album_duplicates (duplicate_album_id);

CREATE INDEX idx_tracks_album_id ON tracks (album_id);

CREATE INDEX idx_album_artists_artist_id ON album_artists (artist_id);

CREATE INDEX idx_album_genres_genre_id ON album_genres (genre_id);

CREATE INDEX idx_album_descriptors_descriptor_id ON album_descriptors (descriptor_id);

CREATE INDEX idx_credits_artist_id ON credits (artist_id);

CREATE INDEX idx_credits_album_id ON credits (album_id);

CREATE INDEX idx_credit_roles_role_id ON credit_roles (role_id);

CREATE INDEX idx_album_languages_language_id ON album_languages (language_id);