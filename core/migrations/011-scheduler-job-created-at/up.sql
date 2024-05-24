-- Step 1: create temporary table with new column
CREATE TABLE temp_scheduler_jobs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  next_execution DATETIME NOT NULL,
  last_execution DATETIME,
  interval_seconds INTEGER,
  payload BLOB,
  claimed_at DATETIME,
  priority INTEGER NOT NULL DEFAULT 2,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
); 

-- Step 2: copy data from old table to new table
INSERT INTO temp_scheduler_jobs (id, name, next_execution, last_execution, interval_seconds, payload)
SELECT id, name, next_execution, last_execution, interval_seconds, payload FROM scheduler_jobs;

-- Step 3: drop old table
DROP TABLE scheduler_jobs;

-- Step 4: rename new table to old table
ALTER TABLE temp_scheduler_jobs RENAME TO scheduler_jobs;