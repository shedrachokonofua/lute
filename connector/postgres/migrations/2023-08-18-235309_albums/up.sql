CREATE TABLE lute_artists (
  file_name TEXT PRIMARY KEY,
  name TEXT NOT NULL
);
CREATE TABLE lute_albums (
  file_name TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  rating FLOAT NOT NULL,
  rating_count INT NOT NULL,
  primary_genres TEXT [] NOT NULL DEFAULT '{}',
  secondary_genres TEXT [] NOT NULL DEFAULT '{}',
  descriptors TEXT [] NOT NULL DEFAULT '{}',
  release_date DATE,
  languages TEXT [] NOT NULL DEFAULT '{}'
);
CREATE TABLE lute_albums_artists (
  artist_file_name TEXT NOT NULL,
  album_file_name TEXT NOT NULL,
  PRIMARY KEY (artist_file_name, album_file_name),
  FOREIGN KEY (artist_file_name) REFERENCES lute_artists(file_name) ON DELETE CASCADE,
  FOREIGN KEY (album_file_name) REFERENCES lute_albums(file_name) ON DELETE CASCADE
);
CREATE TABLE lute_tracks (
  album_file_name TEXT NOT NULL,
  name TEXT NOT NULL,
  duration_seconds INT,
  rating FLOAT,
  position TEXT,
  PRIMARY KEY (album_file_name, name),
  FOREIGN KEY (album_file_name) REFERENCES lute_albums(file_name) ON DELETE CASCADE
);
CREATE TABLE lute_credits (
  artist_file_name TEXT NOT NULL,
  album_file_name TEXT NOT NULL,
  roles TEXT [] NOT NULL DEFAULT '{}',
  PRIMARY KEY (artist_file_name, album_file_name),
  FOREIGN KEY (artist_file_name) REFERENCES lute_artists(file_name) ON DELETE CASCADE,
  FOREIGN KEY (album_file_name) REFERENCES lute_albums(file_name) ON DELETE CASCADE
);