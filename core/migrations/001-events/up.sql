CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  correlation_id TEXT DEFAULT NULL,
  causation_id TEXT DEFAULT NULL,
  event TEXT NOT NULL,
  metadata TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  stream TEXT NOT NULL
);
CREATE INDEX idx_stream_id ON events(stream, id);