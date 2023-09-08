CREATE TABLE event_subscriber_cursors (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  subscriber_id TEXT NOT NULL,
  stream TEXT NOT NULL,
  cursor INTEGER NOT NULL,
  UNIQUE(subscriber_id, stream)
);
CREATE INDEX idx_subscriber_id_stream ON event_subscriber_cursors(subscriber_id, stream);