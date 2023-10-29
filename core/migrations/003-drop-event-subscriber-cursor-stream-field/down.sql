-- Down Migration
PRAGMA foreign_keys = off;

CREATE TEMPORARY TABLE Backup AS
SELECT
  subscriber_id,
  cursor
FROM
  event_subscriber_cursors;

DROP TABLE event_subscriber_cursors;

CREATE TABLE event_subscriber_cursors (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  subscriber_id TEXT NOT NULL,
  stream TEXT NOT NULL,
  cursor INTEGER NOT NULL,
  UNIQUE(subscriber_id, stream)
);

INSERT INTO
  event_subscriber_cursors (subscriber_id, stream, cursor)
SELECT
  subscriber_id,
  '' AS stream,
  cursor
FROM
  Backup;

DROP TABLE Backup;

PRAGMA foreign_keys = on;