CREATE TABLE albums (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_name TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  rating REAL NOT NULL,
  rating_count INTEGER NOT NULL,
  release_date DATE,
  cover_image_url TEXT
);

CREATE TABLE album_duplicates (
  original_album_id INTEGER NOT NULL,
  duplicate_album_id INTEGER NOT NULL UNIQUE,
  PRIMARY KEY (original_album_id, duplicate_album_id),
  FOREIGN KEY (original_album_id) REFERENCES albums(id) ON DELETE CASCADE,
  FOREIGN KEY (duplicate_album_id) REFERENCES albums(id) ON DELETE CASCADE
);

CREATE TABLE artists (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  file_name TEXT NOT NULL UNIQUE
);

CREATE TABLE tracks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  album_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  duration_seconds INTEGER,
  rating REAL,
  position TEXT,
  FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE
);

CREATE TABLE album_artists (
  album_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,
  PRIMARY KEY (album_id, artist_id),
  FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
);

CREATE TABLE genres (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE album_genres (
  album_id INTEGER NOT NULL,
  genre_id INTEGER NOT NULL,
  is_primary BOOLEAN NOT NULL,
  PRIMARY KEY (album_id, genre_id, is_primary),
  FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE,
  FOREIGN KEY (genre_id) REFERENCES genres(id) ON DELETE CASCADE
);

CREATE TABLE descriptors (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE album_descriptors (
  album_id INTEGER NOT NULL,
  descriptor_id INTEGER NOT NULL,
  PRIMARY KEY (album_id, descriptor_id),
  FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE,
  FOREIGN KEY (descriptor_id) REFERENCES descriptors(id) ON DELETE CASCADE
);

CREATE TABLE credits (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  album_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,
  FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
);

CREATE TABLE roles (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE credit_roles (
  credit_id INTEGER NOT NULL,
  role_id INTEGER NOT NULL,
  PRIMARY KEY (credit_id, role_id),
  FOREIGN KEY (credit_id) REFERENCES credits(id) ON DELETE CASCADE,
  FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

CREATE TABLE languages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE album_languages (
  album_id INTEGER NOT NULL,
  language_id INTEGER NOT NULL,
  PRIMARY KEY (album_id, language_id),
  FOREIGN KEY (album_id) REFERENCES albums(id) ON DELETE CASCADE,
  FOREIGN KEY (language_id) REFERENCES languages(id) ON DELETE CASCADE
);