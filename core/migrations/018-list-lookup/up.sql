CREATE TABLE list_segments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_name TEXT NOT NULL UNIQUE,
  root_file_name TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE list_segment_siblings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  list_segment_id INTEGER NOT NULL,
  sibling_file_name TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (list_segment_id) REFERENCES list_segments(id),
  UNIQUE (list_segment_id, sibling_file_name)
);

CREATE TABLE list_segment_albums (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  list_segment_id INTEGER NOT NULL,
  file_name TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (list_segment_id) REFERENCES list_segments(id),
  UNIQUE (list_segment_id, file_name)
);

CREATE TABLE list_lookups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  root_file_name TEXT NOT NULL UNIQUE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);