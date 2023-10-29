-- Up Migration
-- Delete rows with subscriber_id values that appear with multiple stream values
WITH DuplicateSubscribers AS (
  SELECT
    subscriber_id
  FROM
    event_subscriber_cursors
  GROUP BY
    subscriber_id
  HAVING
    COUNT(DISTINCT stream) > 1
)
DELETE FROM
  event_subscriber_cursors
WHERE
  subscriber_id IN (
    SELECT
      subscriber_id
    FROM
      DuplicateSubscribers
  );

-- Drop the stream column and set subscriber_id as the primary key
PRAGMA foreign_keys = off;

CREATE TEMPORARY TABLE Backup AS
SELECT
  subscriber_id,
  cursor
FROM
  event_subscriber_cursors;

DROP TABLE event_subscriber_cursors;

CREATE TABLE event_subscriber_cursors (
  subscriber_id TEXT PRIMARY KEY,
  cursor INTEGER NOT NULL
);

INSERT INTO
  event_subscriber_cursors (subscriber_id, cursor)
SELECT
  subscriber_id,
  cursor
FROM
  Backup;

DROP TABLE Backup;

PRAGMA foreign_keys = on;