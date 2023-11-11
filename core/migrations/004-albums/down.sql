-- Start by dropping tables with foreign keys
DROP TABLE IF EXISTS album_duplicates;

DROP TABLE IF EXISTS album_artists;

DROP TABLE IF EXISTS album_genres;

DROP TABLE IF EXISTS album_descriptors;

DROP TABLE IF EXISTS credit_roles;

DROP TABLE IF EXISTS credits;

DROP TABLE IF EXISTS album_languages;

DROP TABLE IF EXISTS tracks;

-- Then drop the referenced tables
DROP TABLE IF EXISTS albums;

DROP TABLE IF EXISTS artists;

DROP TABLE IF EXISTS genres;

DROP TABLE IF EXISTS descriptors;

DROP TABLE IF EXISTS languages;

DROP TABLE IF EXISTS roles;