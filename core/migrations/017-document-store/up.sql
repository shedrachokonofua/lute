CREATE TABLE document_store (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  collection TEXT NOT NULL,
  key TEXT NOT NULL,
  json BLOB NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  expires_at DATETIME DEFAULT NULL,
  UNIQUE(collection, key)
);

CREATE INDEX idx_document_store_lookup ON document_store (collection, key, expires_at);