ALTER TABLE events ADD COLUMN key TEXT;
CREATE UNIQUE INDEX event_stream_key ON events (stream, key);