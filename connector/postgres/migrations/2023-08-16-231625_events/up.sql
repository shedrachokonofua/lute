CREATE TABLE lute_events (
  id VARCHAR PRIMARY KEY,
  stream_id VARCHAR NOT NULL,
  payload JSONB NOT NULL,
  event_timestamp BIGINT NOT NULL,
  saved_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE INDEX idx_lute_events_payload ON lute_events USING gin(payload);
CREATE INDEX idx_lute_events_stream_id ON lute_events (stream_id);