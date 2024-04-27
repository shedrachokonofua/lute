CREATE TABLE key_value_store (
  key TEXT PRIMARY KEY,
  value BLOB NOT NULL,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  expires_at DATETIME
);