CREATE INDEX idx_scheduler_jobs_name_next_execution_priority_id
  ON scheduler_jobs (name, next_execution, priority, id);
CREATE INDEX idx_scheduler_jobs_claimed_at
  ON scheduler_jobs (claimed_at);
