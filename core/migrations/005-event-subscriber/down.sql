CREATE TABLE temp_event_subscriber_cursors (
  subscriber_id TEXT PRIMARY KEY,
  cursor INTEGER NOT NULL
);

INSERT INTO
  temp_event_subscriber_cursors (subscriber_id, cursor)
SELECT
  id,
  cursor
FROM
  event_subscribers;

DROP TABLE event_subscribers;

ALTER TABLE
  temp_event_subscriber_cursors RENAME TO event_subscriber_cursors;