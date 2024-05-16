CREATE TABLE scheduler_jobs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  next_execution DATETIME NOT NULL,
  last_execution DATETIME,
  interval_seconds INTEGER,
  payload BLOB
);

CREATE INDEX idx_scheduler_jobs_next_execution ON scheduler_jobs (next_execution);