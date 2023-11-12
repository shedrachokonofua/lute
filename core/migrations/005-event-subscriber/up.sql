CREATE TABLE temp_event_subscribers (
  id TEXT PRIMARY KEY,
  cursor INTEGER NOT NULL,
  status INTEGER NOT NULL DEFAULT 1
);

INSERT INTO
  temp_event_subscribers (id, cursor)
SELECT
  subscriber_id,
  cursor
FROM
  event_subscriber_cursors;

DROP TABLE event_subscriber_cursors;

ALTER TABLE
  temp_event_subscribers RENAME TO event_subscribers;